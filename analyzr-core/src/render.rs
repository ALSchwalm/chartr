use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use svg::node::element as Svg;
use svg::node::element::path::Data;
use svg::Document;

use crate::event::{ActorId, EventKind, EventStore};

const APPROX_FONT_HEIGHT: f64 = 15.0;

#[derive(Deserialize, Serialize)]
struct RenderOpts {
    us_per_line: u64,
    sublines: u32,
    us_per_pixel: u32,
    pixels_per_actor: f64,
    actor_margin: f64,
    actor_name_padding: f64,
    top_margin: f64,
    side_margin: f64,
    heading: String,
}

impl Default for RenderOpts {
    fn default() -> Self {
        Self {
            us_per_line: Duration::from_secs(1).as_micros() as u64,
            sublines: 10,
            us_per_pixel: 10000,
            pixels_per_actor: 20.0,
            actor_margin: 0.5,
            actor_name_padding: 5.0,
            top_margin: 20.0,
            side_margin: 20.0,
            heading: "".into(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct RendererBuilder {
    opts: RenderOpts,
}

impl RendererBuilder {
    pub fn build(self) -> Renderer {
        Renderer { opts: self.opts }
    }

    pub fn heading(mut self, heading: impl AsRef<str>) -> Self {
        self.opts.heading = heading.as_ref().into();
        self
    }
}

#[derive(Deserialize, Serialize)]
pub struct Renderer {
    opts: RenderOpts,
}

impl Renderer {
    fn us_to_pixel(&self, us: i64) -> f64 {
        us as f64 / self.opts.us_per_pixel as f64
    }

    fn render_line_time(&self, us: i64) -> String {
        // TODO: we probably shouldn't hard code this as seconds
        let seconds = us as f64 / 1_000_000.0;
        let fac = us as f64 % 1_000_000.0;
        format!("{seconds}.{fac}")
    }

    fn calculate_heading_height(&self) -> f64 {
        let heading_start = self.opts.top_margin + APPROX_FONT_HEIGHT;
        let lines = self.opts.heading.lines().count() as f64;
        let heading_end = heading_start + lines * APPROX_FONT_HEIGHT +
            // Skip a couple of "lines" after the text of the heading
            2.0 * APPROX_FONT_HEIGHT;
        heading_end
    }

    fn render_heading(&self, mut output: Document) -> Result<Document> {
        let mut current_y = self.opts.top_margin + APPROX_FONT_HEIGHT;
        for line in self.opts.heading.lines() {
            let text = Svg::Text::new(line)
                .set("class", "heading")
                .set("x", self.opts.side_margin)
                .set("y", current_y);
            current_y += APPROX_FONT_HEIGHT;
            output = output.add(text);
        }

        Ok(output)
    }

    fn render_actor(
        &self,
        mut output: Svg::Group,
        y: f64,
        box_width: f64,
        first_event_pixel: f64,
        events: &EventStore,
        actor: ActorId,
    ) -> Result<Svg::Group> {
        let mut g = Svg::Group::new().set("class", "actor");

        let mut actor_start: Option<i64> = None;
        for (i, event) in events
            .events_for(&actor)
            .with_context(|| "Failed to get actor events")?
            .enumerate()
        {
            let (start, duration) = match event.kind {
                EventKind::Span(start, duration) => (start, duration),
                //TODO: handle instants
                _ => unimplemented!(),
            };

            // Only draw the actor label at the start of the first span
            if i == 0 {
                actor_start = Some(start);
            }

            let width = match duration {
                Some(duration) => self.us_to_pixel(duration as i64),
                None => (first_event_pixel + box_width) - self.us_to_pixel(start),
            };

            let mut state = Svg::Rectangle::new()
                .set("class", "span")
                .set("width", width)
                .set(
                    "height",
                    self.opts.pixels_per_actor - 2.0 * self.opts.actor_margin,
                )
                .set("x", self.us_to_pixel(start))
                .set("y", y + self.opts.actor_margin);

            let attrs = state.get_attributes_mut();
            for (key, value) in event.fields.clone().into_iter() {
                let current = attrs.entry(key.clone()).or_insert("".into()).clone();
                attrs.insert(key, format!("{value} {current}").into());
            }

            g = g.add(state);
        }

        if let Some(start) = actor_start {
            let actor_name = events.get_actor(&actor);

            let (class, padding) =
                if self.us_to_pixel(start) < (first_event_pixel + box_width) / 2.0 {
                    ("left", self.opts.actor_name_padding)
                } else {
                    ("right", -self.opts.actor_name_padding)
                };

            let text = Svg::Text::new(actor_name.identity.clone())
                .set("class", class)
                .set("x", self.us_to_pixel(start) + padding)
                // Assume the font is probably about 80% of the line
                // height.
                .set("y", y + self.opts.pixels_per_actor * 0.8);

            g = g.add(text);
        }

        output = output.add(g);
        Ok(output)
    }

    fn render_lines(
        &self,
        mut g: Svg::Group,
        first_event_time: i64,
        last_event_time: i64,
        box_height: f64,
    ) -> Result<Svg::Group> {
        let first_bar = first_event_time - (first_event_time % self.opts.us_per_line as i64);
        let last_bar = last_event_time + (last_event_time % self.opts.us_per_line as i64);

        let step = self.opts.us_per_line as usize / self.opts.sublines as usize;
        for x in (first_bar..=last_bar).step_by(step) {
            let scaled_x = self.us_to_pixel(x);

            let data = Data::new()
                .move_to((scaled_x, 0))
                .line_by((0, box_height))
                .close();

            let mut path = Svg::Path::new().set("d", data);

            if x.unsigned_abs() % self.opts.us_per_line == 0 {
                let text = Svg::Text::new(self.render_line_time(x))
                    .set("class", "label")
                    .set("x", scaled_x)
                    .set("y", -5);
                g = g.add(text);
            } else {
                path = path.set("class", "subline");
            }

            g = g.add(path);
        }

        Ok(g)
    }

    fn render_css(&self, document: Document) -> Result<Document> {
        let defs = Svg::Definitions::new().add(Svg::Style::new(
            "
        rect.span      { opacity: 0.7; }
        g.actor:hover rect { opacity: 1.0; }
        path           { stroke: rgb(64,64,64); stroke-width: 1; }
        path.subline   { stroke: rgb(224,224,224); stroke-width: 0.7; }
        text           { font-family: Verdana, Helvetica; font-size: 14px; }
        text.left      { font-family: Verdana, Helvetica; font-size: 14px; text-anchor: start; }
        text.right     { font-family: Verdana, Helvetica; font-size: 14px; text-anchor: end; }
        text.label     { font-size: 10px; }",
        ));
        Ok(document.add(defs))
    }

    pub fn render(&self, path: impl AsRef<Path>, events: EventStore) -> Result<()> {
        // First, determine how many lines we need
        let first_event_time = events
            .all_events()
            .min_by_key(|e| e.start_time())
            .map(|e| {
                if e.start_time() > 0 {
                    0
                } else {
                    e.start_time()
                }
            })
            .unwrap_or(0);

        let last_event_time = events
            .all_events()
            .filter_map(|e| e.end_time())
            .max()
            .unwrap_or(0);

        // Gather the relevant actors for height calculation and such
        let mut actors = events
            .actors()
            .filter_map(|actor| events.events_for(&actor).ok()?.next().map(|e| (actor, e)))
            .collect::<Vec<_>>();

        actors.sort_by_key(|(_, event)| event.start_time());

        let heading_height = self.calculate_heading_height();

        // TODO: consider heading width may be greater than box width
        let box_width = self.us_to_pixel(last_event_time - first_event_time);
        let box_height = actors.len() as f64 * self.opts.pixels_per_actor;

        let mut document = Document::new()
            .set("width", box_width + 2.0 * self.opts.side_margin)
            .set("height", box_height + heading_height + self.opts.top_margin);

        let serialized = svg::node::Comment::new(serde_json::to_string(&(self, &events))?);
        document = document.add(serialized);

        document = self.render_css(document)?;
        document = self.render_heading(document)?;

        let start_x = self.opts.side_margin
            + if first_event_time < 0 {
                -self.us_to_pixel(first_event_time)
            } else {
                0.0
            };

        let mut g = Svg::Group::new().set(
            "transform",
            format!("translate({start_x}, {heading_height})"),
        );
        g = self.render_lines(g, first_event_time, last_event_time, box_height)?;

        let mut y = 0.0;
        for (actor, _) in actors.into_iter() {
            g = self
                .render_actor(
                    g,
                    y,
                    box_width,
                    self.us_to_pixel(first_event_time),
                    &events,
                    actor,
                )
                .with_context(|| "Failed to render actor events")?;

            y += self.opts.pixels_per_actor;
        }

        document = document.add(g);

        svg::save(path, &document).with_context(|| "Failed to save svg")
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            opts: RenderOpts::default(),
        }
    }
}

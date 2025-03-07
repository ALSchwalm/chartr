use anyhow::{bail, Result};
use std::path::Path;

pub mod event;
pub mod render;

pub fn load(path: impl AsRef<Path>) -> Result<(render::Renderer, event::EventStore)> {
    let mut buffer = String::new();
    let parser = svg::open(path, &mut buffer)?;

    for item in parser {
        match item {
            svg::parser::Event::Comment(c) => {
                // The svg crate keeps the added "<!-- " and " -->"
                // text, so strip it before we deserialize
                return Ok(serde_json::from_str(&c[5..c.len() - 4])?);
            }
            _ => (),
        }
    }

    bail!("Failed to find comment to parse")
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::{event::*, render::*};
    use std::{collections::BTreeMap, time::Duration};

    #[test]
    fn test_render() {
        let r = RendererBuilder::default()
            .heading("My Heading\nanother line")
            .build();

        let mut context = EventStore::default();

        let actor = context.register_actor(Actor::new("myproc")).unwrap();

        let actor2 = context.register_actor(Actor::new("myproc2")).unwrap();

        context
            .add_event(
                &actor,
                Event {
                    fields: BTreeMap::from([("fill".into(), "#AB7C94".into())]),
                    kind: EventKind::Span(
                        Duration::from_millis(3500).as_micros() as i64,
                        Some(Duration::from_millis(750).as_micros() as u32),
                    ),
                    value: "start1".into(),
                    tooltip: None
                },
            )
            .unwrap();

        context
            .add_event(
                &actor,
                Event {
                    fields: BTreeMap::from([("fill".into(), "#AB7C94".into())]),
                    kind: EventKind::Span(
                        Duration::from_millis(1500).as_micros() as i64,
                        Some(Duration::from_millis(750).as_micros() as u32),
                    ),
                    value: "other1".into(),
                    tooltip: None
                },
            )
            .unwrap();

        context
            .add_event(
                &actor2,
                Event {
                    fields: BTreeMap::from([("fill".into(), "#AB7C94".into())]),
                    kind: EventKind::Span(
                        -(Duration::from_millis(5000).as_micros() as i64),
                        Some(Duration::from_millis(2000).as_micros() as u32),
                    ),
                    value: "start2".into(),
                    tooltip: None
                },
            )
            .unwrap();

        r.render("/tmp/foo.svg", context).unwrap();

        let (r2, events2) = load("/tmp/foo.svg").unwrap();
        r2.render("/tmp/foo2.svg", events2).unwrap();
    }
}

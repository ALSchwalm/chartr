use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum EventKind {
    Span(i64, Option<u32>),
    Instant(i64),
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Event {
    pub fields: BTreeMap<String, String>,
    pub kind: EventKind,
    pub value: String,
    pub tooltip: Option<String>
}

impl Event {
    pub fn start_time(&self) -> i64 {
        match self.kind {
            EventKind::Span(start, _) => start,
            EventKind::Instant(instant) => instant,
        }
    }

    pub fn end_time(&self) -> Option<i64> {
        match self.kind {
            EventKind::Span(start, Some(duration)) => Some(start + duration as i64),
            EventKind::Span(_, None) => None,
            EventKind::Instant(instant) => Some(instant),
        }
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.start_time(), self.end_time()).cmp(&(other.start_time(), other.end_time()))
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Actor {
    pub identity: String,
    pub tooltip: Option<String>
}

impl Actor {
    pub fn new(identity: impl AsRef<str>) -> Self {
        Self {
            identity: identity.as_ref().to_owned(),
            tooltip: None
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventStore {
    actors: BTreeMap<ActorId, Actor>,
    events: BTreeMap<ActorId, BTreeSet<Event>>,
}

pub type ActorId = String;

impl EventStore {
    pub fn register_actor(&mut self, actor: Actor) -> Result<ActorId> {
        let actor_id = actor.identity.clone();
        ensure!(
            self.actors.insert(actor_id.clone(), actor).is_none(),
            "Actor already registered"
        );
        ensure!(self
            .events
            .insert(actor_id.clone(), BTreeSet::new())
            .is_none());
        Ok(actor_id)
    }

    pub fn add_event(&mut self, actor: &ActorId, event: Event) -> Result<()> {
        let Some(events) = self.events.get_mut(actor) else {
            bail!("Unknown actor id: {}", actor);
        };

        events.insert(event);
        Ok(())
    }

    pub fn all_events(&self) -> impl Iterator<Item = &Event> {
        self.events.values().flatten()
    }

    pub fn events_for(&self, actor: &ActorId) -> Result<impl Iterator<Item = &Event>> {
        let Some(events) = self.events.get(actor) else {
            bail!("Unknown actor id: {}", actor);
        };

        Ok(events.iter())
    }

    pub fn actors<'a>(&'a self) -> impl Iterator<Item = ActorId> + 'a {
        self.events.keys().cloned()
    }

    pub fn get_actor(&self, id: &ActorId) -> &Actor {
        self.actors.get(id).expect("Invalid actor id")
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self {
            actors: BTreeMap::new(),
            events: BTreeMap::new(),
        }
    }
}

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use std::borrow::Cow;

use super::error::JobError;
use crate::{entity::*, primitives::JobId};

#[derive(Clone, Eq, Hash, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct JobType(Cow<'static, str>);
impl JobType {
    pub const fn new(job_type: &'static str) -> Self {
        JobType(Cow::Borrowed(job_type))
    }
}
impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobEvent {
    Initialized {
        id: JobId,
        job_type: JobType,
        name: String,
        description: Option<String>,
        config: serde_json::Value,
    },
}

impl EntityEvent for JobEvent {
    type EntityId = JobId;
    fn event_table_name() -> &'static str {
        "job_events"
    }
}

#[derive(Builder)]
#[builder(pattern = "owned", build_fn(error = "EntityError"))]
pub struct Job {
    pub id: JobId,
    pub name: String,
    pub job_type: JobType,
    pub description: Option<String>,
    config: serde_json::Value,
    pub(super) _events: EntityEvents<JobEvent>,
}

impl Job {
    pub fn config<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.config.clone())
    }
}

impl Entity for Job {
    type Event = JobEvent;
}

impl TryFrom<EntityEvents<JobEvent>> for Job {
    type Error = EntityError;

    fn try_from(events: EntityEvents<JobEvent>) -> Result<Self, Self::Error> {
        let mut builder = JobBuilder::default();
        for event in events.iter() {
            let JobEvent::Initialized {
                id,
                name,
                job_type,
                description,
                config,
            } = event;
            builder = builder
                .id(*id)
                .name(name.clone())
                .job_type(job_type.clone())
                .description(description.clone())
                .config(config.clone());
        }
        builder._events(events).build()
    }
}

#[derive(Builder, Debug)]
pub struct NewJob {
    #[builder(setter(into))]
    pub id: JobId,
    #[builder(setter(into))]
    pub(super) name: String,
    #[builder(setter(into))]
    pub(super) job_type: JobType,
    #[builder(setter(into), default)]
    pub(super) description: Option<String>,
    #[builder(setter(custom))]
    pub(super) config: serde_json::Value,
}

impl NewJob {
    pub fn builder() -> NewJobBuilder {
        NewJobBuilder::default()
    }

    pub(super) fn initial_events(self) -> EntityEvents<JobEvent> {
        EntityEvents::init(
            self.id,
            [JobEvent::Initialized {
                id: self.id,
                name: self.name,
                job_type: self.job_type,
                description: self.description,
                config: self.config,
            }],
        )
    }
}

impl NewJobBuilder {
    pub fn config<C: serde::Serialize>(&mut self, config: C) -> Result<&mut Self, JobError> {
        self.config =
            Some(serde_json::to_value(config).map_err(JobError::CouldNotSerializeConfig)?);
        Ok(self)
    }
}
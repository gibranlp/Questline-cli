use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Project, Task, TaskPriority};

pub const PORTABLE_FORMAT: &str = "questline-campaign-template";
pub const PORTABLE_VERSION: u32 = 1;
pub const MAX_TEMPLATE_BYTES: usize = 256 * 1024;
pub const MAX_CAMPAIGN_NAME_CHARS: usize = 80;
pub const MAX_QUESTS: usize = 100;
pub const MAX_STEPS_PER_QUEST: usize = 100;
pub const MAX_TOTAL_STEPS: usize = 1_000;

/// A reusable, local-first Campaign blueprint. Templates contain no identity,
/// completion, assignment, or Fellowship data; those are created for the user
/// when the blueprint is instantiated.
#[derive(Debug, Clone, Copy)]
pub struct CampaignTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub quests: &'static [QuestTemplate],
}

#[derive(Debug, Clone, Copy)]
pub struct QuestTemplate {
    pub title: &'static str,
    pub description: &'static str,
    pub priority: TaskPriority,
    pub steps: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableCampaignFile {
    pub format: String,
    pub version: u32,
    pub campaign: PortableCampaign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableCampaign {
    pub name: String,
    pub description: Option<String>,
    pub quests: Vec<PortableQuest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableQuest {
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub steps: Vec<String>,
}

const SOFTWARE_RELEASE_QUESTS: &[QuestTemplate] = &[
    QuestTemplate {
        title: "Define the release",
        description: "Agree on scope, success criteria, and ownership before implementation.",
        priority: TaskPriority::High,
        steps: &["Confirm scope", "Define success criteria", "Assign owners"],
    },
    QuestTemplate {
        title: "Build and validate",
        description: "Complete the work and verify the release candidate.",
        priority: TaskPriority::High,
        steps: &[
            "Complete implementation",
            "Run quality checks",
            "Prepare release notes",
        ],
    },
    QuestTemplate {
        title: "Launch and observe",
        description: "Ship deliberately, monitor the result, and capture follow-up work.",
        priority: TaskPriority::Medium,
        steps: &[
            "Deploy the release",
            "Monitor key signals",
            "Record follow-up quests",
        ],
    },
];

const CONTENT_SPRINT_QUESTS: &[QuestTemplate] = &[
    QuestTemplate {
        title: "Shape the brief",
        description: "Clarify the audience, message, channel, and desired outcome.",
        priority: TaskPriority::High,
        steps: &[
            "Define the audience",
            "Choose the core message",
            "Set the publishing goal",
        ],
    },
    QuestTemplate {
        title: "Create the content",
        description: "Move from outline through review to an approved final version.",
        priority: TaskPriority::High,
        steps: &[
            "Draft the outline",
            "Create the first version",
            "Review and revise",
        ],
    },
    QuestTemplate {
        title: "Publish and learn",
        description: "Release the work and record what the response teaches the team.",
        priority: TaskPriority::Medium,
        steps: &[
            "Publish",
            "Share with the audience",
            "Capture results and lessons",
        ],
    },
];

const EVENT_LAUNCH_QUESTS: &[QuestTemplate] = &[
    QuestTemplate {
        title: "Plan the gathering",
        description: "Define the purpose, format, date, budget, and responsibilities.",
        priority: TaskPriority::High,
        steps: &[
            "Confirm purpose and format",
            "Set date and budget",
            "Assign responsibilities",
        ],
    },
    QuestTemplate {
        title: "Prepare participants",
        description: "Ready the venue or platform, invitations, and event materials.",
        priority: TaskPriority::High,
        steps: &[
            "Prepare venue or platform",
            "Send invitations",
            "Finalize materials",
        ],
    },
    QuestTemplate {
        title: "Run and close",
        description: "Deliver the event, follow up with participants, and capture lessons.",
        priority: TaskPriority::Medium,
        steps: &["Run the event", "Send follow-up", "Document outcomes"],
    },
];

pub const TEMPLATES: &[CampaignTemplate] = &[
    CampaignTemplate {
        id: "software-release",
        name: "Software Release",
        description: "Plan, validate, and launch a software release with a small team.",
        quests: SOFTWARE_RELEASE_QUESTS,
    },
    CampaignTemplate {
        id: "content-sprint",
        name: "Content Sprint",
        description: "Take a focused piece of content from brief through publication.",
        quests: CONTENT_SPRINT_QUESTS,
    },
    CampaignTemplate {
        id: "event-launch",
        name: "Event Launch",
        description: "Coordinate a workshop, meetup, launch, or other team event.",
        quests: EVENT_LAUNCH_QUESTS,
    },
];

pub fn get(index: usize) -> Option<&'static CampaignTemplate> {
    TEMPLATES.get(index)
}

pub fn unique_campaign_name(
    base_name: &str,
    existing_lowercase_names: &std::collections::HashSet<String>,
) -> String {
    if !existing_lowercase_names.contains(&base_name.to_lowercase()) {
        return base_name.to_string();
    }

    for suffix in 2usize.. {
        let suffix = format!(" {suffix}");
        let available = MAX_CAMPAIGN_NAME_CHARS.saturating_sub(suffix.chars().count());
        let truncated = base_name.chars().take(available).collect::<String>();
        let candidate = format!("{}{suffix}", truncated.trim_end());
        if !existing_lowercase_names.contains(&candidate.to_lowercase()) {
            return candidate;
        }
    }
    unreachable!("an unbounded numeric suffix must eventually be unique")
}

impl CampaignTemplate {
    pub fn portable(self) -> PortableCampaignFile {
        PortableCampaignFile {
            format: PORTABLE_FORMAT.to_string(),
            version: PORTABLE_VERSION,
            campaign: PortableCampaign {
                name: self.name.to_string(),
                description: Some(self.description.to_string()),
                quests: self
                    .quests
                    .iter()
                    .map(|quest| PortableQuest {
                        title: quest.title.to_string(),
                        description: Some(quest.description.to_string()),
                        priority: quest.priority,
                        steps: quest.steps.iter().map(|step| (*step).to_string()).collect(),
                    })
                    .collect(),
            },
        }
    }
}

impl PortableCampaignFile {
    pub fn parse(json: &str) -> Result<Self> {
        if json.len() > MAX_TEMPLATE_BYTES {
            return Err(anyhow!(
                "Campaign template exceeds the {} KiB limit",
                MAX_TEMPLATE_BYTES / 1024
            ));
        }
        let file: Self = serde_json::from_str(json)?;
        file.validate()?;
        Ok(file)
    }

    pub fn to_pretty_json(&self) -> Result<String> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn validate(&self) -> Result<()> {
        if self.format != PORTABLE_FORMAT {
            return Err(anyhow!("Not a Questline Campaign template"));
        }
        if self.version != PORTABLE_VERSION {
            return Err(anyhow!(
                "Unsupported Campaign template version {} (expected {})",
                self.version,
                PORTABLE_VERSION
            ));
        }
        validate_text(
            "Campaign name",
            &self.campaign.name,
            MAX_CAMPAIGN_NAME_CHARS,
            false,
        )?;
        if let Some(description) = self.campaign.description.as_deref() {
            validate_text("Campaign description", description, 2_000, true)?;
        }
        if self.campaign.quests.is_empty() {
            return Err(anyhow!(
                "A Campaign template must contain at least one Quest"
            ));
        }
        if self.campaign.quests.len() > MAX_QUESTS {
            return Err(anyhow!(
                "Campaign template contains more than {MAX_QUESTS} Quests"
            ));
        }

        let mut total_steps = 0usize;
        for (quest_idx, quest) in self.campaign.quests.iter().enumerate() {
            validate_text(
                &format!("Quest {} title", quest_idx + 1),
                &quest.title,
                120,
                false,
            )?;
            if let Some(description) = quest.description.as_deref() {
                validate_text(
                    &format!("Quest {} description", quest_idx + 1),
                    description,
                    2_000,
                    true,
                )?;
            }
            if quest.steps.len() > MAX_STEPS_PER_QUEST {
                return Err(anyhow!(
                    "Quest {} contains more than {MAX_STEPS_PER_QUEST} steps",
                    quest_idx + 1
                ));
            }
            total_steps = total_steps.saturating_add(quest.steps.len());
            if total_steps > MAX_TOTAL_STEPS {
                return Err(anyhow!(
                    "Campaign template contains more than {MAX_TOTAL_STEPS} total steps"
                ));
            }
            for (step_idx, step) in quest.steps.iter().enumerate() {
                validate_text(
                    &format!("Quest {} step {}", quest_idx + 1, step_idx + 1),
                    step,
                    120,
                    false,
                )?;
            }
        }
        Ok(())
    }

    /// Export only reusable structure. Identity, sharing, assignments, status,
    /// completion, dates, comments, notes, activity, and encryption state never
    /// enter the portable format.
    pub fn from_campaign(project: &Project, tasks: &[Task]) -> Result<Self> {
        let mut parents = tasks
            .iter()
            .filter(|task| task.parent_task_id.is_none())
            .collect::<Vec<_>>();
        parents.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.title.cmp(&right.title))
        });
        let parent_ids = parents
            .iter()
            .map(|task| task.id)
            .collect::<std::collections::HashSet<_>>();
        if tasks.iter().any(|task| {
            task.parent_task_id
                .is_some_and(|parent_id| !parent_ids.contains(&parent_id))
        }) {
            return Err(anyhow!(
                "Campaign contains a step whose parent Quest is missing"
            ));
        }

        let quests = parents
            .into_iter()
            .map(|parent| {
                let mut steps = tasks
                    .iter()
                    .filter(|task| task.parent_task_id == Some(parent.id))
                    .collect::<Vec<_>>();
                steps.sort_by(|left, right| {
                    left.created_at
                        .cmp(&right.created_at)
                        .then_with(|| left.title.cmp(&right.title))
                });
                PortableQuest {
                    title: parent.title.clone(),
                    description: parent.description.clone(),
                    priority: parent.priority,
                    steps: steps.into_iter().map(|step| step.title.clone()).collect(),
                }
            })
            .collect();
        let file = Self {
            format: PORTABLE_FORMAT.to_string(),
            version: PORTABLE_VERSION,
            campaign: PortableCampaign {
                name: project.name.clone(),
                description: project.description.clone(),
                quests,
            },
        };
        file.validate()?;
        Ok(file)
    }

    pub fn materialize(
        &self,
        campaign_name: String,
        owner_identity: String,
        owner_username: String,
    ) -> Result<(Project, Vec<(Task, Vec<Task>)>)> {
        self.validate()?;
        validate_text(
            "Campaign name",
            &campaign_name,
            MAX_CAMPAIGN_NAME_CHARS,
            false,
        )?;
        let project_id = Uuid::new_v4();
        let now = Utc::now();
        let project = Project {
            id: project_id,
            name: campaign_name,
            description: self.campaign.description.clone(),
            created_at: now,
            updated_at: now,
            archived: false,
            completed: false,
            owner_identity: Some(owner_identity.clone()),
            owner_username: Some(owner_username.clone()),
            is_shared: false,
        };
        let task_trees = self
            .campaign
            .quests
            .iter()
            .map(|quest| {
                let parent_id = Uuid::new_v4();
                let parent = Task {
                    id: parent_id,
                    project_id: Some(project_id),
                    title: quest.title.clone(),
                    description: quest.description.clone(),
                    due_date: None,
                    set_date: None,
                    completed: false,
                    priority: quest.priority,
                    created_at: now,
                    updated_at: now,
                    owner_identity: Some(owner_identity.clone()),
                    owner_username: Some(owner_username.clone()),
                    parent_task_id: None,
                    xp_awarded: false,
                    recurrence: None,
                };
                let steps = quest
                    .steps
                    .iter()
                    .map(|title| Task {
                        id: Uuid::new_v4(),
                        project_id: Some(project_id),
                        title: title.clone(),
                        description: None,
                        due_date: None,
                        set_date: None,
                        completed: false,
                        priority: TaskPriority::Medium,
                        created_at: now,
                        updated_at: now,
                        owner_identity: Some(owner_identity.clone()),
                        owner_username: Some(owner_username.clone()),
                        parent_task_id: Some(parent_id),
                        xp_awarded: false,
                        recurrence: None,
                    })
                    .collect();
                (parent, steps)
            })
            .collect();
        Ok((project, task_trees))
    }
}

fn validate_text(label: &str, value: &str, max_chars: usize, allow_newlines: bool) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{label} cannot be empty"));
    }
    if value.chars().count() > max_chars {
        return Err(anyhow!("{label} exceeds the {max_chars}-character limit"));
    }
    if value.chars().any(|character| {
        character.is_control() && !(allow_newlines && matches!(character, '\n' | '\r' | '\t'))
    }) {
        return Err(anyhow!("{label} contains unsupported control characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_campaign_templates_have_stable_unique_content() {
        let mut ids = std::collections::HashSet::new();
        for template in TEMPLATES {
            assert!(ids.insert(template.id));
            assert!(!template.name.trim().is_empty());
            assert!(!template.description.trim().is_empty());
            assert!(!template.quests.is_empty());
            for quest in template.quests {
                assert!(!quest.title.trim().is_empty());
                assert!(!quest.steps.is_empty());
                assert!(quest.steps.iter().all(|step| !step.trim().is_empty()));
            }
        }
    }

    #[test]
    fn portable_template_round_trip_is_versioned_and_identity_free() {
        let file = TEMPLATES[0].portable();
        let json = file.to_pretty_json().unwrap();
        let parsed = PortableCampaignFile::parse(&json).unwrap();
        assert_eq!(parsed, file);
        assert!(json.contains(PORTABLE_FORMAT));
        assert!(!json.contains("owner_identity"));
        assert!(!json.contains("completed"));
        assert!(!json.contains("is_shared"));
    }

    #[test]
    fn portable_template_rejects_unknown_fields_versions_and_limits() {
        assert!(PortableCampaignFile::parse(&" ".repeat(MAX_TEMPLATE_BYTES + 1)).is_err());

        let valid = TEMPLATES[0].portable().to_pretty_json().unwrap();
        let with_identity = valid.replacen(
            "\"campaign\": {",
            "\"campaign\": {\"owner_identity\": \"stolen-key\",",
            1,
        );
        assert!(PortableCampaignFile::parse(&with_identity).is_err());

        let mut unknown_version = TEMPLATES[0].portable();
        unknown_version.version = PORTABLE_VERSION + 1;
        assert!(unknown_version.validate().is_err());

        let mut too_many_quests = TEMPLATES[0].portable();
        too_many_quests.campaign.quests = vec![
            PortableQuest {
                title: "Quest".to_string(),
                description: None,
                priority: TaskPriority::Medium,
                steps: Vec::new(),
            };
            MAX_QUESTS + 1
        ];
        assert!(too_many_quests.validate().is_err());

        let mut control_character = TEMPLATES[0].portable();
        control_character.campaign.name = "Bad\0Campaign".to_string();
        assert!(control_character.validate().is_err());
    }

    #[test]
    fn duplicate_campaign_names_remain_unique_within_the_length_limit() {
        let base = "a".repeat(MAX_CAMPAIGN_NAME_CHARS);
        let mut existing = std::collections::HashSet::from([base.to_lowercase()]);
        let second = unique_campaign_name(&base, &existing);
        assert_eq!(second.chars().count(), MAX_CAMPAIGN_NAME_CHARS);
        assert!(second.ends_with(" 2"));
        existing.insert(second.to_lowercase());
        let third = unique_campaign_name(&base, &existing);
        assert!(third.ends_with(" 3"));
        assert_eq!(third.chars().count(), MAX_CAMPAIGN_NAME_CHARS);
    }

    #[test]
    fn campaign_export_keeps_structure_but_drops_private_runtime_state() {
        let project_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let now = Utc::now();
        let project = Project {
            id: project_id,
            name: "Private Fellowship Work".to_string(),
            description: Some("Reusable structure".to_string()),
            created_at: now,
            updated_at: now,
            archived: false,
            completed: true,
            owner_identity: Some("private-owner-key".to_string()),
            owner_username: Some("Secret Hero".to_string()),
            is_shared: true,
        };
        let task = |id, title: &str, parent_task_id| Task {
            id,
            project_id: Some(project_id),
            title: title.to_string(),
            description: None,
            due_date: Some(now),
            set_date: None,
            completed: true,
            priority: TaskPriority::High,
            created_at: now,
            updated_at: now,
            owner_identity: Some("private-assignee-key".to_string()),
            owner_username: Some("Private Companion".to_string()),
            parent_task_id,
            xp_awarded: true,
            recurrence: None,
        };
        let tasks = vec![
            task(parent_id, "Reusable Quest", None),
            task(Uuid::new_v4(), "Reusable step", Some(parent_id)),
        ];

        let file = PortableCampaignFile::from_campaign(&project, &tasks).unwrap();
        let json = file.to_pretty_json().unwrap();
        assert!(json.contains("Reusable Quest"));
        assert!(json.contains("Reusable step"));
        for private_value in [
            "private-owner-key",
            "Secret Hero",
            "private-assignee-key",
            "Private Companion",
            "completed",
            "due_date",
            "is_shared",
        ] {
            assert!(!json.contains(private_value));
        }

        let (imported_project, trees) = file
            .materialize(
                "Imported Campaign".to_string(),
                "new-owner".to_string(),
                "New Hero".to_string(),
            )
            .unwrap();
        assert!(!imported_project.completed);
        assert!(!imported_project.is_shared);
        assert!(trees.iter().all(|(parent, steps)| {
            !parent.completed
                && parent.due_date.is_none()
                && steps
                    .iter()
                    .all(|step| !step.completed && step.due_date.is_none())
        }));
    }
}

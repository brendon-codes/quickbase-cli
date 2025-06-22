pub const SKILL_NAMES: [&str; 2] = ["quickbase-api", "quickbase-cli"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkillFile {
    pub skill_name: &'static str,
    pub relative_path: &'static str,
    pub contents: &'static str,
}

pub const FILES: &[SkillFile] = &[
    SkillFile {
        skill_name: "quickbase-api",
        relative_path: "SKILL.md",
        contents: include_str!("../.codex/skills/quickbase-api/SKILL.md"),
    },
    SkillFile {
        skill_name: "quickbase-api",
        relative_path: "references/argument-model.md",
        contents: include_str!("../.codex/skills/quickbase-api/references/argument-model.md"),
    },
    SkillFile {
        skill_name: "quickbase-api",
        relative_path: "references/quickbase-rest-api.yaml",
        contents: include_str!("../.codex/skills/quickbase-api/references/quickbase-rest-api.yaml"),
    },
    SkillFile {
        skill_name: "quickbase-api",
        relative_path: "references/operation-index.md",
        contents: include_str!("../.codex/skills/quickbase-api/references/operation-index.md"),
    },
    SkillFile {
        skill_name: "quickbase-cli",
        relative_path: "SKILL.md",
        contents: include_str!("../.codex/skills/quickbase-cli/SKILL.md"),
    },
    SkillFile {
        skill_name: "quickbase-cli",
        relative_path: "references/commands.md",
        contents: include_str!("../.codex/skills/quickbase-cli/references/commands.md"),
    },
    SkillFile {
        skill_name: "quickbase-cli",
        relative_path: "references/config.md",
        contents: include_str!("../.codex/skills/quickbase-cli/references/config.md"),
    },
];

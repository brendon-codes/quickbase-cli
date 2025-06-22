use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::{
    config,
    error::{QuickbaseCliError, Result},
};

use super::state::MockDataset;

#[derive(Clone, Debug)]
pub struct MockStorage {
    root: PathBuf,
}

impl MockStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn reset(&self) -> Result<()> {
        for managed_path in [self.root.join("state.json"), self.root.join("realms")] {
            if !managed_path.exists() {
                continue;
            }

            if managed_path.is_dir() {
                fs::remove_dir_all(&managed_path).with_context(|| {
                    format!("failed to reset mock data at {}", managed_path.display())
                })?;
            } else {
                fs::remove_file(&managed_path).with_context(|| {
                    format!("failed to reset mock data at {}", managed_path.display())
                })?;
            }
        }

        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create mock data root {}", self.root.display()))?;
        self.persist(&MockDataset::default())
    }

    pub fn persist(&self, dataset: &MockDataset) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create mock data root {}", self.root.display()))?;
        write_json(self.root.join("state.json"), &dataset.counters)?;

        let realms_dir = self.root.join("realms");
        fs::create_dir_all(&realms_dir).with_context(|| {
            format!(
                "failed to create mock realms directory {}",
                realms_dir.display()
            )
        })?;

        for (realm_id, realm) in &dataset.realms {
            let realm_dir = realms_dir.join(safe_segment(realm_id));
            fs::create_dir_all(&realm_dir).with_context(|| {
                format!(
                    "failed to create mock realm directory {}",
                    realm_dir.display()
                )
            })?;
            write_json(realm_dir.join("groups.json"), &realm.groups)?;
            write_json(realm_dir.join("users.json"), &realm.users)?;

            let apps_dir = realm_dir.join("apps");
            fs::create_dir_all(&apps_dir).with_context(|| {
                format!(
                    "failed to create mock apps directory {}",
                    apps_dir.display()
                )
            })?;

            for (app_id, app) in &realm.apps {
                let app_dir = apps_dir.join(safe_segment(app_id));
                fs::create_dir_all(&app_dir).with_context(|| {
                    format!("failed to create mock app directory {}", app_dir.display())
                })?;
                write_json(app_dir.join("app.json"), &app.app)?;

                let tables_dir = app_dir.join("tables");
                fs::create_dir_all(&tables_dir).with_context(|| {
                    format!(
                        "failed to create mock tables directory {}",
                        tables_dir.display()
                    )
                })?;

                for (table_id, table) in &app.tables {
                    let table_dir = tables_dir.join(safe_segment(table_id));
                    fs::create_dir_all(&table_dir).with_context(|| {
                        format!(
                            "failed to create mock table directory {}",
                            table_dir.display()
                        )
                    })?;
                    write_json(table_dir.join("table.json"), &table.table)?;
                    write_json(table_dir.join("fields.json"), &table.fields)?;
                    write_json(table_dir.join("records.json"), &table.records)?;
                }
            }
        }

        Ok(())
    }
}

pub fn default_data_dir() -> Result<PathBuf> {
    Ok(config::repo_root()?.join(".quickbase").join("data"))
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        QuickbaseCliError::Other(anyhow::anyhow!(
            "mock storage path has no parent: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "failed to create mock storage directory {}",
            parent.display()
        )
    })?;
    let text = serde_json::to_string_pretty(value)
        .with_context(|| format!("failed to serialize mock data {}", path.display()))?;
    fs::write(&path, format!("{text}\n"))
        .with_context(|| format!("failed to write mock data {}", path.display()))?;
    Ok(())
}

fn safe_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' => character,
            _ => '_',
        })
        .collect()
}

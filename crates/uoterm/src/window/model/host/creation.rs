//! What the character creation reads from the client files: the
//! professions, the skill names, the words about the start towns and the
//! text numbers. The pages use them through
//! `uoterm_view::model::creation`.

use std::path::Path;
use uoterm_nav::ClilocData;
use uoterm_view::model::creation::CreationFiles;

/// Reads what the creation needs from the client files. With none, the
/// creation has no professions and no skill names.
pub fn read_creation_files(uopath: Option<&Path>) -> CreationFiles {
    let Some(dir) = uopath else {
        return CreationFiles::default();
    };
    CreationFiles {
        words: ClilocData::open(dir).ok(),
        ..read_creation_tables(dir)
    }
}

/// Reads what the creation needs from the client files in `dir`, but the
/// text database. The web server takes the words from the text database it
/// keeps.
pub fn read_creation_tables(dir: &Path) -> CreationFiles {
    CreationFiles {
        professions: uoterm_nav::read_professions(dir),
        skill_names: uoterm_nav::read_skills(dir)
            .map(|skills| skills.into_iter().map(|skill| skill.name).collect())
            .unwrap_or_default(),
        town_texts: uoterm_nav::read_city_texts(dir).unwrap_or_default(),
        words: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_no_client_files_the_creation_has_no_professions() {
        let files = read_creation_files(None);
        assert!(files.professions.top().is_empty() && files.skill_names.is_empty());
    }
}

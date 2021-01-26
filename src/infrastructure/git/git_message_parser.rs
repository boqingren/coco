use std::collections::HashMap;

use regex::{Captures, Regex};

use crate::domain::git::coco_commit::FileChange;
use crate::domain::git::CocoCommit;

lazy_static! {
    static ref REV: Regex = Regex::new(r"\[(?P<rev>[\d|a-f]{5,12})\]").unwrap();
    static ref AUTHOR: Regex = Regex::new(r"(?P<author>.*?)\s\d{10}").unwrap();
    static ref DATE: Regex = Regex::new(r"(?P<date>\d{10})").unwrap();
    static ref CHANGES: Regex =
        Regex::new(r"(?P<deleted>[\d-]+)[\t\s]+(?P<added>[\d-]+)[\t\s]+(?P<filename>.*)").unwrap();
    static ref COMPLEXMOVEREGSTR: Regex = Regex::new(r"(.*)\{(.*)\s=>\s(.*)\}(.*)").unwrap();
    static ref BASICMOVEREGSTR: Regex = Regex::new(r"(.*)\s=>\s(.*)").unwrap();
    static ref CHANGEMODEL: Regex =
        Regex::new(r"\s(\w{1,6})\s(mode 100(\d){3})?\s?(.*)(\s\(\d{2}%\))?").unwrap();
}

pub struct GitMessageParser {
    current_commit: CocoCommit,
    current_file_change: Vec<FileChange>,
    commits: Vec<CocoCommit>,
    current_file_change_map: HashMap<String, FileChange>,
}

impl Default for GitMessageParser {
    fn default() -> Self {
        GitMessageParser {
            current_commit: Default::default(),
            current_file_change: vec![],
            commits: vec![],
            current_file_change_map: Default::default(),
        }
    }
}

impl GitMessageParser {
    pub fn to_commit_message(str: &str) -> Vec<CocoCommit> {
        let split = str.split("\n");
        let mut parser = GitMessageParser::default();
        for line in split {
            parser.parse_log_by_line(line)
        }

        parser.commits
    }

    pub fn parse_log_by_line(&mut self, text: &str) {
        let change_mode = "";
        let find_rev = REV.captures(text);
        if let Some(captures) = find_rev {
            let commit = GitMessageParser::create_commit(text, &captures);
            self.current_commit = commit
        } else if let Some(caps) = CHANGES.captures(text) {
            let (filename, file_change) = GitMessageParser::create_file_change(caps);
            self.current_file_change_map.insert(filename, file_change);
        } else if let Some(caps) = CHANGEMODEL.captures(text) {
            // todo
        } else if self.current_commit.rev != "" {
            for (_filename, change) in &self.current_file_change_map {
                self.current_file_change.push(change.clone());
            }

            self.current_commit.changes = self.current_file_change.clone();
            self.commits.push(self.current_commit.clone());

            // self.current_commit = CocoCommit::default();
            self.current_file_change_map.clear();
        }
    }

    fn create_file_change(caps: Captures) -> (String, FileChange) {
        let filename = caps["filename"].to_string();
        let file_change = FileChange {
            added: caps["added"].parse::<i32>().unwrap(),
            deleted: caps["deleted"].parse::<i32>().unwrap(),
            file: filename.clone(),
            mode: "".to_string(),
        };
        (filename, file_change)
    }

    fn create_commit(text: &str, captures: &Captures) -> CocoCommit {
        let cap_rev = &captures["rev"];
        let without_rev = text
            .split(&format!("[{}] ", cap_rev))
            .collect::<Vec<&str>>()[1];

        let author = &AUTHOR.captures(without_rev).unwrap()["author"];
        let without_author = without_rev
            .split(&format!("{} ", author))
            .collect::<Vec<&str>>()[1];

        let date_str = &DATE.captures(without_author).unwrap()["date"];
        let without_date = without_author
            .split(&format!("{} ", date_str))
            .collect::<Vec<&str>>()[1];

        let message = without_date;

        let date = date_str.parse::<i64>().unwrap();
        CocoCommit {
            branch: "".to_string(),
            rev: cap_rev.to_string(),
            author: author.to_string(),
            committer: "".to_string(),
            date,
            message: message.to_string(),
            changes: vec![],
        }
    }
}

#[cfg(test)]
mod test {
    use crate::infrastructure::git::git_message_parser::GitMessageParser;

    #[test]
    pub fn should_success_parse_one_line_log() {
        let input = "[828fe39523] Rossen Stoyanchev 1575388800 Consistently use releaseBody in DefaultWebClient
5       3       spring-webflux/core/main/java/org/springframework/web/reactive/function/client/ClientResponse.java
1       1       spring-webflux/core/main/java/org/springframework/web/reactive/function/client/DefaultWebClient.java
9       3       spring-webflux/core/main/java/org/springframework/web/reactive/function/client/WebClient.java
6       11      core/docs/asciidoc/web/webflux-webclient.adoc
";

        let commits = GitMessageParser::to_commit_message(input);
        assert_eq!(1, commits.len());
        assert_eq!("828fe39523", commits[0].rev);
        assert_eq!("Rossen Stoyanchev", commits[0].author);
        assert_eq!(1575388800, commits[0].date);
        assert_eq!(
            "Consistently use releaseBody in DefaultWebClient",
            commits[0].message
        );
        assert_eq!(4, commits[0].changes.len())
    }

    #[test]
    pub fn should_success_parse_multiple_line_log() {
        let input = "[d00f0124d] Phodal Huang 1575388800 update files
0       0       core/domain/bs/BadSmellApp.go

[1d00f0124b] Phodal Huang 1575388800 update files
1       1       cmd/bs.go
0       0       core/domain/bs/BadSmellApp.go

[d00f04111b] Phodal Huang 1575388800 refactor: move bs to adapter
1       1       cmd/bs.go
5       5       core/{domain => adapter}/bs/BadSmellApp.go

[d00f01214b] Phodal Huang 1575388800 update files
1       1       cmd/bs.go
0       0       core/adapter/bs/BadSmellApp.go
";

        let commits = GitMessageParser::to_commit_message(input);
        assert_eq!(4, commits.len());
    }
}
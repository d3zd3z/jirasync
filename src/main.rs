//! JiraSync
//!
//! This program is starting with a single purpose in life, to capture the things I'm working on
//! that are recorded in other JIRA systems, and update a summary in the description of a ticket on
//! a work internal JIRA.

extern crate goji;
extern crate hyper;
extern crate netrc;

use goji::{Credentials, Issue, Jira, SearchOptions};
use hyper::Client;
use netrc::Netrc;
use std::fs::File;
use std::io::BufReader;
use std::env;

// use std::error;
// use std::result;

// Grumble, goji's error doesn't implement Error, so can't be used with '?'.
// TODO: Submit a pull request to this project to fix this.

// A generic boxed error.
// type Result<T> = result::Result<T, Box<error::Error + Send + Sync>>;

// TODO: Store credentials in a dotfile or some other store.

fn main() {
    let client = Client::new();
    let host = "runtimeco.atlassian.net";
    let jira = Jira::new(format!("https://{}/", host),
                         netrc_lookup(&host),
                         &client);

    let q =
        "(fixVersion = 1.1 OR fixVersion = 1.0)
         AND assignee = david.brown
         ORDER BY priority DESC, updated DESC";
    let mut rows = query(&jira, q,
                         vec!["summary", "assignee", "status", "fixVersions"]);

    // Keys should be sorted first by issue number, and then by fixVersion.  The sort is stable, so
    // this will result in the desired grouped order.
    rows.sort_by_key(|a| num_of_key(&a.key));
    rows.sort_by(|a, b| nice_versions(a).cmp(&nice_versions(b)));

    println!("Issues at [https://runtimeco.atlassian.net/projects/MCUB]\n");
    println!("||Issue||Description||Vers||Status||");
    for row in &rows {
        println!("|[{}|{}]|{}|{}|{}|",
                 row.key,
                 row.self_link,
                 escape(row.fields["summary"].as_str().unwrap()),
                 nice_versions(row),
                 decode_status(row));
    }
    println!("\nQuery: {{panel}}{}{{panel}}", q);
}

fn decode_status(issue: &Issue) -> String {
    let name = issue.fields["status"]
        .find("name").unwrap()
        .as_str().unwrap();
    let color = match name {
        "Open" => "red",
        "Resolved" => "green",
        "Closed" => "green",
        _ => "blue",
    };
    format!("{{color:{}}}{}{{color}}", color, name)
}

// An aggressive escape, to avoid confusing JIRA.
fn escape(text: &str) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        if "-*_?[]{}".contains(ch) {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}

// Format the fixVersions nicely
fn nice_versions(issue: &Issue) -> String {
    let vers: Vec<_> = issue.fields["fixVersions"].as_array().unwrap()
        .iter().map(|v| {
            v.find("name").unwrap().as_str().unwrap()
        }).collect();
    vers.join(",")
}

// Get the numeric key out of a JIRA formatted issue NAME-nnn.
fn num_of_key(key: &str) -> usize {
    let pos = key.find('-').expect("key should have '-'");
    key[pos+1..].parse().unwrap()
}

// Perform a JIRA query, returning the specified fields, and collecting all of the results
// (performing multiple queries if necessary).
fn query<J, F>(jira: &Jira, jql: J, fields: Vec<F>) -> Vec<Issue>
    where J: Into<String> + Clone,
          F: Into<String>
{
    let search = jira.search();
    let mut sopt = SearchOptions::builder();
    sopt.fields(fields);

    let mut result = vec![];

    let mut pos = 0;
    loop {
        sopt.start_at(pos);
        let opt = sopt.build();
        let mut res = search.list(jql.clone(), &opt).unwrap();
        println!("total: {}, num: {}", res.total, res.issues.len());
        pos += res.issues.len() as u64;
        result.append(&mut res.issues);
        if pos >= res.total {
            break;
        }
    }

    result
}

fn netrc_lookup(host: &str) -> Credentials {
    let home = env::home_dir().expect("Cannot find home dir");
    let netrc = home.join(".netrc");
    let fd = BufReader::new(File::open(&netrc).expect("Opening ~/.netrc"));
    let nrc = Netrc::parse(fd).expect("Parser error in ~/.netrc");
    for (h, machine) in nrc.hosts {
        if h == host {
            return Credentials::Basic(machine.login,
                                      machine.password.expect("Needs password"));
        }
    }
    panic!("Host not found in netrc");
}

//! JiraSync
//!
//! This program is starting with a single purpose in life, to capture the things I'm working on
//! that are recorded in other JIRA systems, and update a summary in the description of a ticket on
//! a work internal JIRA.

extern crate env_logger;
extern crate futures;
extern crate goji;
extern crate hyper;
extern crate hyper_tls;
extern crate netrc;
extern crate serde_json;
extern crate tokio_core;

#[macro_use] extern crate serde_derive;

mod github;

use goji::{Credentials, Issue, Jira, SearchOptions};
use netrc::Netrc;
use std::env;
use std::error;
use std::fs::File;
use std::io::BufReader;
use std::result;

use github::query_zephyr;

pub type Result<T> = result::Result<T, Box<error::Error + Send + Sync>>;

struct Project {
    host: String,
    query: String,
    name: String,
}

fn get_project(name: &str) -> Option<Project> {
    match name {
        // Zephyr is obsolete, as it has moved to github.
        "zephyr" => Some(Project {
            host: "jira.zephyrproject.org".to_string(),
            query: "assignee = currentUser()
                    ORDER BY priority DESC, updated DESC".to_string(),
            name: "ZEP".to_string(),
        }),
        "mcuboot" => Some(Project {
            host: "runtimeco.atlassian.net".to_string(),
            query: "(fixVersion = 1.1 OR fixVersion = 1.2) AND
                    assignee = currentUser()
                    ORDER BY priority DESC, updated DESC".to_string(),
            name: "MCUB".to_string(),
        }),
        _ => None,
    }
}

// use std::error;
// use std::result;

// Grumble, goji's error doesn't implement Error, so can't be used with '?'.
// TODO: Submit a pull request to this project to fix this.

// A generic boxed error.
// type Result<T> = result::Result<T, Box<error::Error + Send + Sync>>;

// TODO: Store credentials in a dotfile or some other store.

fn main() {
    env_logger::init().unwrap();

    let project = match env::args().skip(1).next() {
        None => panic!("Expect project name"),
        Some(ref name) if name == "zephyr" => {
            query_zephyr().unwrap();
            return;
        }
        Some(name) => get_project(&name).expect("Valid project name"),
    };

    let jira = Jira::new(format!("https://{}", project.host),
                         netrc_lookup(&project.host)).unwrap();


    let mut rows = query(&jira, project.query.as_ref(),
                         vec!["summary", "assignee", "status", "fixVersions"]);

    // Keys should be sorted first by issue number, and then by fixVersion.  The sort is stable, so
    // this will result in the desired grouped order.
    rows.sort_by_key(|a| num_of_key(&a.key));
    rows.sort_by(|a, b| nice_versions(a).cmp(&nice_versions(b)));

    println!("Issues at [https://{}/projects/{}]\n", project.host, project.name);
    println!("||Issue||Description||Vers||Status||");
    for row in &rows {
        println!("|[{}|{}]|{}|{}|{}|",
                 row.key,
                 row.self_link,
                 escape(row.fields["summary"].as_str().unwrap()),
                 nice_versions(row),
                 decode_status(row));
    }
    println!("\nQuery: {{panel}}{}{{panel}}", project.query);
}

fn decode_status(issue: &Issue) -> String {
    let name = issue.fields["status"]
        .get("name").unwrap()
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
            v.get("name").unwrap().as_str().unwrap()
        }).collect();
    let ver = vers.join(",");
    if ver == "" {
        " ".to_string()
    } else {
        ver
    }
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
        // let mut res = search.list(jql.clone(), &opt).unwrap();
        let res = search.list(jql.clone(), &opt);
        // println!("{:?}", res);
        let mut res = res.unwrap();
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

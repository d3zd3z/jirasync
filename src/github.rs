//! Github API query of issues.

use Result;
use futures::{Future, Stream};
use hyper::{Client, Method, Request};
use hyper::header::{Authorization, Bearer, UserAgent};
use hyper_tls::HttpsConnector;
use serde_json;
use std::env;
use std::io;
use tokio_core::reactor::Core;

static API_URL: &'static str = "https://api.github.com/graphql";
static QUERY: &'static str = r#"
{
  search(query: "assignee:d3zd3z is:issue repo:zephyrproject-rtos/zephyr",
    type: ISSUE, last:100) {
    edges {
      node {
        ... on Issue {
          title
          url
          state
          number
        }
      }
    }
  }
}"#;

// The json request.
#[derive(Debug, Serialize)]
struct Query {
    query: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all="snake_case")]
enum QueryResult {
    Data(Search)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all="snake_case")]
enum Search {
    Search(Edges),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all="snake_case")]
enum Edges {
    Edges(Vec<Edge>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all="snake_case")]
enum Edge {
    Node(Node),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Node {
    title: String,
    state: String,
    url: String,
    number: u64,
}

impl QueryResult {
    /// Get the nodes out of the deep structure.
    fn into_nodes(self) -> Vec<Node> {
        match self {
            QueryResult::Data(search) => match search {
                Search::Search(edges) => match edges {
                    Edges::Edges(nodes) => {
                        nodes.into_iter().map(|node| match node {
                            Edge::Node(node) => node,
                        }).collect()
                    }
                }
            }
        }
    }
}

pub fn query_zephyr() -> Result<()> {
    let token = env::var("GITHUB_TOKEN").expect("Must set GITHUB_TOKEN in env");

    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    let uri = API_URL.parse()?;

    let mut req = Request::new(Method::Post, uri);
    {
        let headers = req.headers_mut();
        headers.set(Authorization(Bearer {
            token: token,
        }));
        headers.set(UserAgent::new("d3zd3z-jirasync"));
    }

    let query = Query {
        query: QUERY.to_string(),
    };
    req.set_body(serde_json::to_string(&query)?);
    /*
    let mut query = serde_json::Map::new();
    query.insert("query".to_string(), Value::String(QUERY.to_string()));
    req.set_body(serde_json::to_string(&Value::Object(query))?);
    */

    let work = client.request(req).and_then(|res| {
        println!("Response: {}", res.status());

        res.body().concat2().and_then(move |body| {
            let v: QueryResult = serde_json::from_slice(&body).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    e
                )
            })?;

            // Flatten the json.
            let nodes = v.into_nodes();

            dump_github(nodes);
            Ok(())
        })
    });
    core.run(work)?;

    Ok(())
}

fn dump_github(mut nodes: Vec<Node>) {
    // Sort by issue number.  TODO: Get milestone.
    nodes.sort_by_key(|a| a.number);

    println!("Issues at [https://github.com/zephyrproject-rtos/zephyr/]\n");
    println!("||Issue||Description||Status||");
    for node in &nodes {
        let color = match node.state.as_ref() {
            "OPEN" => "red",
            "CLOSED" => "green",
            _ => "blue",
        };
        println!("|[{}|{}]|{}|{{color:{}}}{}{{color}}|",
                 node.number,
                 node.url,
                 super::escape(&node.title),
                 color, node.state);
    }
}

package main

import (
	"context"
	"fmt"
	"os"
	"sort"

	"github.com/shurcooL/githubql"
	"golang.org/x/oauth2"
)

// Github queries
type githubProject struct {
	user string
	repo string
	desc string
}

func (prj *githubProject) Desc() string {
	return prj.desc
}

// The information we want from a single issue.
type issue struct {
	Title  githubql.String
	Url    githubql.String
	State  githubql.String
	Number githubql.Int
}

func (prj *githubProject) Query() error {
	// Taken from githubql's README
	src := oauth2.StaticTokenSource(
		&oauth2.Token{AccessToken: os.Getenv("GITHUB_TOKEN")},
	)
	httpClient := oauth2.NewClient(context.Background(), src)

	client := githubql.NewClient(httpClient)

	var query struct {
		Search struct {
			Edges []struct {
				Node struct {
					Issue issue `graphql:"... on Issue"`
				}
			}
			PageInfo struct {
				EndCursor   githubql.String
				HasNextPage githubql.Boolean
			}
		} `graphql:"search(query:$query, type:ISSUE, first:100, after:$cursor)"`
	}

	queryText := fmt.Sprintf("assignee:%s is:issue repo:%s", prj.user, prj.repo)
	variables := map[string]interface{}{
		"query":  githubql.String(queryText),
		"cursor": (*githubql.String)(nil),
	}

	var issues []issue

	for {
		err := client.Query(context.Background(), &query, variables)
		if err != nil {
			return err
		}

		for _, edge := range query.Search.Edges {
			issues = append(issues, edge.Node.Issue)
		}
		// fmt.Printf("Page: %+v\n", query.Search.PageInfo)
		if !query.Search.PageInfo.HasNextPage {
			break
		}
		variables["cursor"] = githubql.NewString(query.Search.PageInfo.EndCursor)
	}

	// Sort by issue number for consistent results.
	sort.Sort(ByIssueNumber(issues))

	fmt.Printf("Issues at [https://github.com/%s/]\n\n", prj.repo)
	fmt.Printf("||Issue||Description||Status||\n")
	for i := range issues {
		var color string
		switch issues[i].State {
		case "OPEN":
			color = "red"
		case "CLOSED":
			color = "green"
		default:
			color = "blue"
		}
		fmt.Printf("|[%d|%s]|%s|{color:%s}%s{color}|\n",
			issues[i].Number,
			issues[i].Url,
			jiraEscape(string(issues[i].Title)),
			color,
			issues[i].State)
	}

	return nil
}

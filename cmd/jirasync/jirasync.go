// JiraSync
//
// This program is starting with a single purpose in life, to capture
// the things I'm working on that are record in other JIRA systems,
// and update a summary in the description of a ticket on a work
// internal JIRA.

package main

import (
	"bytes"
	"flag"
	"fmt"
	"log"
	"os/user"
	"sort"
	"strings"

	"github.com/andygrunwald/go-jira"
	"github.com/bgentry/go-netrc/netrc"
)

type jiraProject struct {
	host  string
	query string
	name  string
	desc  string
}

var projects map[string]*jiraProject

func init() {
	projects = make(map[string]*jiraProject)
	projects["mcuboot"] = &jiraProject{
		host: "runtimeco.atlassian.net",
		query: "(fixVersion = 1.1 OR fixVersion = 1.2) AND " +
			"assignee = currentUser() " +
			"ORDER BY priority DESC, updated DESC",
		desc: "The MCUboot project JIRA",
	}
}

func main() {
	flag.Parse()
	args := flag.Args()
	if len(args) != 1 {
		log.Fatalf("must specify project as sole argument")
	}
	prj, ok := projects[args[0]]
	if !ok {
		showNames()
		log.Fatalf("Unknown project %q", args[0])
	}
	login, err := findPass(prj.host)
	if err != nil {
		log.Fatalf("Unable to find .netrc entry: %s", err)
	}

	issues, err := jiraQuery(prj, login)
	if err != nil {
		log.Fatal(err)
	}

	sortIssues(issues)

	fmt.Printf("Issues at [https://%s/projects/%s]\n\n", prj.host, prj.name)
	fmt.Printf("||Issue||Description||Vers||Status||\n")
	for i := range issues {
		fmt.Printf("|[%s|%s]|%s|%s|%s|\n",
			issues[i].Key,
			issues[i].Self,
			jiraEscape(issues[i].Fields.Summary),
			niceVersion(issues[i].Fields.FixVersions),
			decodeStatus(issues[i].Fields.Status.Name))
	}
	fmt.Printf("\nQuery: {panel}%s{panel}\n", prj.query)

}

func jiraQuery(prj *jiraProject, login *Login) ([]jira.Issue, error) {
	client, err := jira.NewClient(nil, "https://"+prj.host+"/")
	if err != nil {
		return nil, err
	}
	client.Authentication.SetBasicAuth(login.User, login.Password)

	var opt jira.SearchOptions

	result := make([]jira.Issue, 0)
	start := 0

	for {
		opt.Fields = []string{"summary", "assignee", "status", "fixVersions"}
		opt.StartAt = start
		opt.MaxResults = 6
		issues, resp, err := client.Issue.Search(prj.query, &opt)
		if err != nil {
			return nil, err
		}

		result = append(result, issues...)
		// log.Printf("%d issues", len(issues))
		log.Printf("resp: %+v", resp)

		if start+len(issues) == resp.Total {
			break
		}
		start += len(issues)
	}
	return result, nil
}

// Escape some characters in the string so that they can be included
// in JIRA markup text without problems.  This isn't perfect, and in
// the general case, there is no way of doing this.
func jiraEscape(text string) string {
	var result bytes.Buffer
	for _, ch := range text {
		if strings.ContainsRune("-*_?[]{}", ch) {
			result.WriteRune('\\')
		}
		result.WriteRune(ch)
	}
	return result.String()
}

// Convert the fix versions to a nice string.
func niceVersion(fvs []*jira.FixVersion) string {
	var result bytes.Buffer
	for _, fv := range fvs {
		if result.Len() != 0 {
			result.WriteRune(',')
		}
		result.WriteString(fv.Name)
	}
	return result.String()
}

// Format the status with a bit of color.
func decodeStatus(text string) string {
	var color string
	switch text {
	case "Open":
		color = "red"
	case "Resolved", "Closed":
		color = "green"
	default:
		color = "blue"
	}
	return fmt.Sprintf("{color:%s}%s{color}", color, text)
}

type Login struct {
	User, Password string
}

// Determine the given user's jira password for the given host.
func findPass(host string) (*Login, error) {
	usr, err := user.Current()
	if err != nil {
		return nil, err
	}

	path := usr.HomeDir + "/.netrc"
	n, err := netrc.ParseFile(path)
	if err != nil {
		return nil, err
	}

	m := n.FindMachine(host)
	if m == nil {
		return nil, fmt.Errorf("Unable to find host: %s", host)
	}

	return &Login{
		User:     m.Login,
		Password: m.Password,
	}, nil
}

func showNames() {
	longest := 0
	names := make([]string, 0, len(projects))
	for k := range projects {
		names = append(names, k)
		if len(k) > longest {
			longest = len(k)
		}
	}
	sort.Strings(names)
	log.Printf("Possible projects:")
	for _, k := range names {
		log.Printf("    %-*s: %s", longest, k, projects[k].desc)
	}
}

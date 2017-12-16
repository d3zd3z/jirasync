package main

import (
	"log"
	"sort"
	"strconv"
	"strings"

	"github.com/andygrunwald/go-jira"
	"github.com/blang/semver"
)

// Sort issues by fixVersion, and then by id.
func sortIssues(issues []jira.Issue) {
	sort.Stable(ByIdSlice(issues))
	sort.Stable(ByFixVersion(issues))
}

type ByIdSlice []jira.Issue

func (p ByIdSlice) Len() int           { return len(p) }
func (p ByIdSlice) Less(i, j int) bool { return keyNum(&p[i]) < keyNum(&p[j]) }
func (p ByIdSlice) Swap(i, j int)      { p[i], p[j] = p[j], p[i] }

type ByFixVersion []jira.Issue

func (p ByFixVersion) Len() int           { return len(p) }
func (p ByFixVersion) Less(i, j int) bool { return getVer(&p[i]).Compare(getVer(&p[j])) < 0 }
func (p ByFixVersion) Swap(i, j int)      { p[i], p[j] = p[j], p[i] }

// Extract the number part of the issue's key.  Needed for the sort to
// appear in a natural order.  Assumes the issue number follows the
// hyphen in the key.
func keyNum(issue *jira.Issue) int {
	pos := strings.IndexRune(issue.Key, '-')
	if pos < 0 {
		panic("JIRA key doesn't have hyphen")
	}
	key := issue.Key[pos+1 : len(issue.Key)]

	num, err := strconv.Atoi(key)
	if err != nil {
		panic(err)
	}

	return num
}

// JIRA supports multiple fix versions per issue.  In order to sort we
// have to pick one, so we arbitrarily pick the smallest semver.  If
// no versions is set, we'll use "0.0.0".
func getVer(issue *jira.Issue) semver.Version {
	first := true
	base, err := semver.Make("0.0.0")
	if err != nil {
		log.Fatalf("Invalid version: 0.0.0: %s", err)
	}

	for _, fv := range issue.Fields.FixVersions {
		this, err := semver.Make(fv.Name)
		if err != nil {
			// "1.1" is not a valid semantic version.  To
			// make this work, try appending a ".0" to the
			// end.  This won't work with anything else
			// specified (rc, etc), so those will need to
			// have the version fully specified.
			this, err = semver.Make(fv.Name + ".0")
			if err != nil {
				log.Fatalf("Invalid semantic version: %q: %s", fv.Name, err)
			}
		}

		if first {
			base = this
			first = false
			continue
		}

		if this.Compare(base) < 0 {
			base = this
		}
	}

	return base
}

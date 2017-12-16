# Jirasync

This is a little tool I use to extract the status of my Jira and
github tickets.

It is also pulling at my indecision as to whether Go or Rust are best
for a project like this.

## Go

This should build just fine with 'go get':

  go get davidb.org/x/jirasync/cmd/jirasync

  jirasync ...

instructions about the ~/.netrc are the same as the Rust version.

## Rust

You'll need to clone https://github.com/d3zd3z/goji as a subdirectory
of this project, as I have one or more patches to that project needed
to work with this code.

You'll need to install rust to build this.  https://www.rust-lang.org/en-US/install.html

Currently, you'll have to put your JIRA password for Zephyr in
`~/.netrc`:

    machine jira.zephyrproject.org
      login user.name
      password secretgoeshere

github's API allows anonymous queries (but it is rate limited).

You should be able to then do:

    cargo run mcuboot

or

    cargo run zephyr

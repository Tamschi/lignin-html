# lignin-html Changelog

<!-- markdownlint-disable no-trailing-punctuation -->

## next

TODO: Date

**This is a rewrite with changed API!**

It should be still be very easy to upgrade to the new version, since mostly fewer parameters are needed.

* **Breaking:**
  * Upgraded `lignin` dependency to version `"0.1.0"`.
  * Reimplemented the crate from scratch with more convenient API.
  * Much better validation that now aims to only produce syntactically valid HTML, escaping all text where necessary. (partially TODO)
  * Increased minimum supported Rust version from 1.44.0 to 1.46.0
    > required because of `lignin` upgrade in the previous version.

* Revisions:
  * Updated the rust-template version this project is based on,
    which comes with CI improvements and a new SECURITY.md file.

## 0.0.5

2021-01-30

* **Breaking:**
  * Increased minimum supported Rust version from 1.42.0 to 1.44.0
    > required because of `lignin` upgrade in the previous version.

## 0.0.4

2021-01-30

* **Breaking:**
  * Upgraded `lignin` dependency from 0.0.3 to 0.0.5
    > to support fallible allocation/bump object initialisation downstream.

## 0.0.3

2021-01-03

* **Breaking:**
  * Upgraded `lignin` dependency from 0.0.2 to 0.0.3

## 0.0.2

2020-11-30

* **Breaking:**
  * Removed "remnants" features (always enabled now)
  * Upgraded `lignin` dependency from 0.0.1 to 0.0.2

## 0.0.1

2020-10-02

Initial unstable release

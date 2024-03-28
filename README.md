# remote-call

[![Latest Version](https://img.shields.io/crates/v/remote-call.svg)](https://crates.io/crates/remote-call)
[![License](https://img.shields.io/github/license/LorenzoLeonardo/remote-call.svg)](LICENSE-MIT)
[![Documentation](https://docs.rs/remote-call/badge.svg)](https://docs.rs/remote-call)
[![Build Status](https://github.com/LorenzoLeonardo/remote-call/workflows/Rust/badge.svg)](https://github.com/LorenzoLeonardo/remote-call/actions)

This crate is a cross-platform inter-process communication system that manages messages across processes using TCP stream.
It uses JSON formatted stream as a protocol when exchanging messages across processes.

This is also a library for the client-side processes for Rust.
The user application can share the object across the TCP stream.

Supported OS:
- Windows
- Linux
- Mac OS

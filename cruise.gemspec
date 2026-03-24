# frozen_string_literal: true

require_relative "lib/cruise/version"

Gem::Specification.new do |spec|
  spec.name = "cruise"
  spec.version = Cruise::VERSION
  spec.authors = ["Marco Roth"]
  spec.email = ["marco.roth@intergga.ch"]

  spec.summary = "A fast, native file watcher for Ruby"
  spec.description = "Cruise is a Rust-powered file system watcher with native OS integration. Uses FSEvents on macOS and inotify on Linux."
  spec.homepage = "https://github.com/marcoroth/cruise"
  spec.license = "MIT"

  spec.required_ruby_version = ">= 3.2.0"
  spec.require_paths = ["lib"]

  spec.files = Dir[
    "cruise.gemspec",
    "LICENSE.txt",
    "Cargo.toml",
    "Rakefile",
    "lib/**/*.rb",
    "ext/**/*.{rs,toml,rb,lock}"
  ]

  spec.extensions = ["ext/cruise/extconf.rb"]

  spec.metadata["allowed_push_host"] = "https://rubygems.org"
  spec.metadata["rubygems_mfa_required"] = "true"
  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["changelog_uri"] = "#{spec.homepage}/releases"
  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["bug_tracker_uri"] = "#{spec.homepage}/issues"

  spec.add_dependency "rb_sys"
end

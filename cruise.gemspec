# frozen_string_literal: true

require_relative "lib/cruise/version"

Gem::Specification.new do |spec|
  spec.name = "cruise"
  spec.version = Cruise::VERSION
  spec.authors = ["Marco Roth"]
  spec.email = ["marco.roth@intergga.ch"]

  spec.summary = "A fast, OS-native file watcher for Ruby"
  spec.description = "Cruise is a file system watcher built on native OS events. Uses FSEvents on macOS and inotify on Linux."
  spec.homepage = "https://github.com/marcoroth/cruise"
  spec.license = "MIT"

  spec.required_ruby_version = ">= 3.2.0"
  spec.require_paths = ["lib"]

  spec.files = Dir[
    "cruise.gemspec",
    "LICENSE.txt",
    "README.md",
    "Cargo.toml",
    "Cargo.lock",
    "Rakefile",
    "lib/**/*.rb",
    "sig/**/*.rbs",
    "ext/cruise/build.rs",
    "ext/cruise/Cargo.toml",
    "ext/cruise/cbindgen.toml",
    "ext/cruise/cruise.c",
    "ext/cruise/extconf.rb",
    "ext/cruise/include/**/*.h",
    "ext/cruise/src/**/*.rs"
  ]

  spec.extensions = ["ext/cruise/extconf.rb"]

  spec.metadata["allowed_push_host"] = "https://rubygems.org"
  spec.metadata["rubygems_mfa_required"] = "true"
  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["changelog_uri"] = "#{spec.homepage}/releases"
  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["bug_tracker_uri"] = "#{spec.homepage}/issues"
end

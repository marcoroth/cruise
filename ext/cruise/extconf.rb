# frozen_string_literal: true

require "mkmf"
require "fileutils"

crate_dir = __dir__
root_dir = File.expand_path("../..", __dir__)

unless system("cargo --version > /dev/null 2>&1")
  abort <<~MESSAGE

    ERROR: Rust toolchain not found.

    cruise requires the Rust toolchain to compile from source.

    Install Rust: https://rustup.rs

  MESSAGE
end

RUST_TARGETS = {
  "aarch64-linux-gnu" => "aarch64-unknown-linux-gnu",
  "aarch64-linux-musl" => "aarch64-unknown-linux-musl",
  "arm-linux-gnu" => "armv7-unknown-linux-gnueabihf",
  "arm-linux-musl" => "armv7-unknown-linux-musleabihf",
  "arm64-darwin" => "aarch64-apple-darwin",
  "x86_64-darwin" => "x86_64-apple-darwin",
  "x86_64-linux-gnu" => "x86_64-unknown-linux-gnu",
  "x86_64-linux-musl" => "x86_64-unknown-linux-musl",
  "x86-linux-gnu" => "i686-unknown-linux-gnu",
  "x86-linux-musl" => "i686-unknown-linux-musl",
}.freeze

cross_compiling = ENV.key?("RUBY_CC_VERSION")
target_platform = ENV.fetch("CARGO_BUILD_TARGET", nil)

if cross_compiling && target_platform.nil?
  rcd_platform = ENV.fetch("RCD_PLATFORM", "")
  target_platform = RUST_TARGETS[rcd_platform]

  if target_platform.nil?
    ruby_platform = RbConfig::CONFIG["arch"]
    target_platform = RUST_TARGETS.values.find { |t| ruby_platform.include?(t.split("-").first) }
  end
end

workspace_target_dir = File.join(root_dir, "target")
crate_target_dir = File.join(crate_dir, "target")

if target_platform
  puts "cruise: Cross-compiling Rust for target: #{target_platform}"
  system("rustup target add #{target_platform}") || warn("cruise: Failed to add Rust target #{target_platform}")

  cargo_args = "--release --target #{target_platform}"
else
  puts "cruise: Compiling Rust library for native platform..."

  cargo_args = "--release"
end

unless system("cd #{root_dir} && cargo build #{cargo_args}")
  abort "ERROR: Failed to compile cruise from Rust source."
end

lib_dir = if target_platform
            [workspace_target_dir, crate_target_dir]
              .map { |dir| File.join(dir, target_platform, "release") }
              .find { |dir| Dir.exist?(dir) } || File.join(crate_target_dir, target_platform, "release")
          else
            [workspace_target_dir, crate_target_dir]
              .map { |dir| File.join(dir, "release") }
              .find { |dir| Dir.exist?(dir) } || File.join(crate_target_dir, "release")
          end

host_os = target_platform || RbConfig::CONFIG["host_os"]

lib_name = case host_os
           when /darwin/ then "libcruise.dylib"
           when /mingw|mswin|windows/ then "cruise.dll"
           else "libcruise.so"
           end

static_lib = File.join(lib_dir, "libcruise.a")

if File.exist?(static_lib)
  puts "cruise: Static library found at #{static_lib}"
  $LDFLAGS << " #{static_lib}"
else
  lib_path = File.join(lib_dir, lib_name)

  unless File.exist?(lib_path)
    abort "ERROR: cruise library not found at #{static_lib} or #{lib_path}"
  end

  puts "cruise: Shared library found at #{lib_path} (dynamic)"

  $LDFLAGS << " -L#{lib_dir} -lcruise"

  if host_os.match?(/darwin|linux/)
    $LDFLAGS << " -Wl,-rpath,#{lib_dir}"
  end
end

case host_os
when /darwin/
  $LDFLAGS << " -framework CoreServices -framework CoreFoundation -framework Security"
when /linux/
  $LDFLAGS << " -lpthread -ldl -lm"
end

$CFLAGS << " -I#{File.join(__dir__, "include")}"

create_makefile("cruise/cruise")

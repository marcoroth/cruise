# frozen_string_literal: true

require "bundler/gem_tasks"
require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/*_test.rb"]
end

begin
  require "rake/extensiontask"
  require "rb_sys"
  require "rb_sys/extensiontask"

  PLATFORMS = [
    "aarch64-linux-gnu",
    "aarch64-linux-musl",
    "arm-linux-gnu",
    "arm64-darwin",
    "x86_64-darwin",
    "x86_64-linux-gnu",
    "x86_64-linux-musl"
  ].freeze

  RB_SYS_PLATFORM_MAP = {
    "aarch64-linux-gnu" => "aarch64-linux",
    "aarch64-linux-musl" => "aarch64-linux-musl",
    "arm-linux-gnu" => "arm-linux",
    "arm64-darwin" => "arm64-darwin",
    "x86_64-darwin" => "x86_64-darwin",
    "x86_64-linux-gnu" => "x86_64-linux",
    "x86_64-linux-musl" => "x86_64-linux-musl"
  }.freeze

  RbSys::ExtensionTask.new("cruise", Gem::Specification.load("cruise.gemspec")) do |ext|
    ext.lib_dir = "lib/cruise"
    ext.cross_compile = true
    ext.cross_platform = PLATFORMS
  end

  namespace "gem" do
    task "prepare" do
      require "rake_compiler_dock"

      sh "bundle config set cache_all true"

      gemspec_path = File.expand_path("./cruise.gemspec", __dir__)
      spec = eval(File.read(gemspec_path), binding, gemspec_path)

      RakeCompilerDock.set_ruby_cc_version(spec.required_ruby_version.as_list)
    rescue LoadError
      abort "rake_compiler_dock is required for this task"
    end

    PLATFORMS.each do |platform|
      desc "Build all native binary gems in parallel"
      multitask "native" => platform

      desc "Build the native gem for #{platform}"
      task platform => "prepare" do
        rb_sys_platform = RB_SYS_PLATFORM_MAP.fetch(platform)

        RakeCompilerDock.sh(
          "bundle install && bundle exec rake native:#{platform} gem RUBY_CC_VERSION='#{ENV.fetch("RUBY_CC_VERSION", nil)}'",
          platform: platform,
          image: "rbsys/#{rb_sys_platform}:#{RbSys::VERSION}"
        )
      end
    end
  end
rescue LoadError => e
  warn "WARNING: Failed to load extension tasks: #{e.message}"

  desc "Compile task not available (rake-compiler not installed)"
  task :compile do
    abort "rake-compiler is required: #{e.message}\n\nRun: bundle install"
  end
end

task default: [:compile, :test]

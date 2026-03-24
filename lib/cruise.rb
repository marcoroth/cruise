# frozen_string_literal: true

require_relative "cruise/version"

begin
  ruby_version = RUBY_VERSION.split(".")[0..1].join(".")

  begin
    require "cruise/#{ruby_version}/cruise"
  rescue LoadError
    require "cruise/cruise"
  end
rescue LoadError => e
  raise LoadError, "Failed to load Cruise native extension: #{e.message}"
end

module Cruise
  DEFAULT_DEBOUNCE = 0.1

  class << self
    def watch(*args, glob: nil, debounce: DEFAULT_DEBOUNCE, only: nil, callback: nil, &block)
      callback = block || callback
      paths = args.flatten.grep(String)

      raise ArgumentError, "Cruise.watch requires at least one path" if paths.empty?
      raise ArgumentError, "Cruise.watch requires a block or callback" unless callback

      glob_patterns = glob ? Array(glob) : []
      only_kinds = only ? Array(only).map(&:to_s) : []

      _watch(paths, callback, debounce.to_f, glob_patterns, only_kinds)
    end
  end
end

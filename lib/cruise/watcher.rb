# frozen_string_literal: true

module Cruise
  class Watcher
    #: (*paths paths, ?glob: globs?, ?debounce: debounce, ?only: kinds?) -> void
    def initialize(*paths, glob: nil, debounce: DEFAULT_DEBOUNCE, only: nil)
      paths = paths.flatten.grep(String)

      raise ArgumentError, "Cruise::Watcher requires at least one path" if paths.empty?

      glob_patterns = glob ? Array(glob) : [] #: Array[String]
      only_kinds = only ? Array(only).map(&:to_s) : [] #: Array[String]

      initialize_native(paths, debounce.to_f, glob_patterns, only_kinds)
    end

    #: () -> IO
    def io
      raise NotImplementedError, "Cruise::Watcher#io is provided by the native extension"
    end

    #: () -> Event?
    def poll
      raise NotImplementedError, "Cruise::Watcher#poll is provided by the native extension"
    end

    #: () -> void
    def close
      raise NotImplementedError, "Cruise::Watcher#close is provided by the native extension"
    end

    private

    #: (Array[String] paths, Float debounce, Array[String] globs, Array[String] only_kinds) -> void
    def initialize_native(_paths, _debounce, _globs, _only_kinds)
      raise NotImplementedError, "Cruise::Watcher#initialize_native is provided by the native extension"
    end
  end
end

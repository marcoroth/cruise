# frozen_string_literal: true

require "io/wait"

require_relative "cruise/version"
require_relative "cruise/event"
require_relative "cruise/watcher"

begin
  ruby_version = RUBY_VERSION.split(".").take(2).join(".")

  begin
    require "cruise/#{ruby_version}/cruise"
  rescue LoadError
    require "cruise/cruise"
  end
rescue LoadError => e
  raise LoadError, "Failed to load Cruise native extension: #{e.message}"
end

module Cruise
  DEFAULT_DEBOUNCE = 0.1 #: Float

  class << self
    # Watch one or more paths and yield a Cruise::Event for each change.
    #
    # Blocks until interrupted. Waiting happens on the watcher's pipe via
    # IO#wait_readable, so this releases the GVL for other threads and, when a
    # Fiber scheduler is set (e.g. inside Async), yields to other fibers instead
    # of blocking the reactor.
    #: (*paths paths, ?glob: globs?, ?debounce: debounce, ?only: kinds?, ?callback: callback?) ?{ (Event) -> void } -> void
    def watch(*paths, glob: nil, debounce: DEFAULT_DEBOUNCE, only: nil, callback: nil, &block)
      callback = block || callback

      raise ArgumentError, "Cruise.watch requires a block or callback" unless callback

      watcher = Watcher.new(*paths, glob: glob, debounce: debounce, only: only)
      io = watcher.io

      begin
        loop do
          io.wait_readable
          closed = drain_wakeups(io)

          while (event = watcher.poll)
            callback.call(event)
          end

          break if closed
        end
      ensure
        watcher.close
      end
    end

    private

    #: (IO io) -> bool
    def drain_wakeups(io)
      loop { io.read_nonblock(4096) }
    rescue IO::WaitReadable
      false
    rescue IOError
      true
    end
  end
end

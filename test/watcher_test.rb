# frozen_string_literal: true

require "test_helper"
require "fileutils"

class WatcherTest < Minitest::Spec
  def wait_for_event(watcher)
    40.times do
      watcher.io.wait_readable(0.1)

      begin
        watcher.io.read_nonblock(4096)
      rescue IO::WaitReadable, IOError
        nil
      end

      while (event = watcher.poll)
        return event if yield(event)
      end
    end

    nil
  end

  it "exposes a readable IO and a non-blocking poll" do
    directory = Dir.mktmpdir("cruise-watcher")
    watcher = Cruise::Watcher.new(directory, debounce: 0.02)

    begin
      assert_kind_of IO, watcher.io
      assert_nil watcher.poll, "poll should be non-blocking and return nil when idle"

      File.write(File.join(directory, "poll.txt"), "hey")

      event = wait_for_event(watcher) { |candidate| candidate.path.include?("poll.txt") }

      refute_nil event, "expected an event for poll.txt"
      assert_equal "created", event.kind
    ensure
      watcher.close
      FileUtils.rm_rf(directory)
    end
  end

  it "can be driven from a plain Fiber with no scheduler" do
    directory = Dir.mktmpdir("cruise-plain-fiber")
    watcher = Cruise::Watcher.new(directory, debounce: 0.02)
    received = nil
    resumes = 0

    fiber = Fiber.new do
      loop do
        event = watcher.poll

        if event&.path&.include?("plain.txt")
          received = event
          break
        end

        Fiber.yield
      end
    end

    writer = Thread.new do
      sleep 0.05

      File.write(File.join(directory, "plain.txt"), "hey")
    end

    begin
      100.times do
        break if received

        fiber.resume if fiber.alive?
        resumes += 1
        sleep 0.02
      end

      writer.join

      refute_nil received, "expected the event to reach the plain fiber via poll"
      assert_operator resumes, :>, 1, "fiber should have yielded while idle, not blocked on the first resume"
    ensure
      watcher.close
      FileUtils.rm_rf(directory)
    end
  end

  it "poll returns nil after close" do
    directory = Dir.mktmpdir("cruise-watcher-close")
    watcher = Cruise::Watcher.new(directory, debounce: 0.02)

    watcher.close

    assert_nil watcher.poll
    assert_predicate watcher.io, :closed?
    watcher.close # idempotent
  ensure
    FileUtils.rm_rf(directory)
  end
end

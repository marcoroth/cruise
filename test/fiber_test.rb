# frozen_string_literal: true

require "test_helper"
require "fileutils"
require "async"

class FiberSchedulerTest < Minitest::Spec
  it "yields to other fibers while waiting, under the Async scheduler" do
    directory = Dir.mktmpdir("cruise-fiber")
    ticks = 0
    received = nil

    Async do |task|
      ticker = task.async do
        100.times do
          sleep 0.005
          ticks += 1
        end
      end

      task.async do
        sleep 0.05
        File.write(File.join(directory, "async.txt"), "hello")
      end

      catch(:stop) do
        Cruise.watch(directory, debounce: 0.02) do |event|
          received = event
          throw(:stop) if event.path.include?("async.txt")
        end
      end

      ticker.stop
    end

    refute_nil received, "expected the watcher to deliver the event under Async"
    assert_operator ticks, :>, 0, "reactor was blocked: no other fiber ran while Cruise.watch waited"
  ensure
    FileUtils.rm_rf(directory)
  end
end

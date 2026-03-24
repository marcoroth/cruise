# frozen_string_literal: true

require "test_helper"
require "fileutils"

class CruiseTest < Minitest::Spec
  it "has a version" do
    assert_kind_of String, Cruise::VERSION
    refute_empty Cruise::VERSION
  end

  it "raises when no block or callback is given" do
    assert_raises(ArgumentError) do
      Cruise.watch(".")
    end
  end

  it "raises when path does not exist" do
    assert_raises(ArgumentError) do
      Cruise.watch("/nonexistent/path/that/does/not/exist") { |event| event }
    end
  end

  it "accepts a single string path" do
    directory = Dir.mktmpdir("cruise-test")
    detected_events = []

    Thread.new do
      sleep 0.5
      File.write(File.join(directory, "new_file.txt"), "hello")
    end

    catch(:stop) do
      Cruise.watch(directory) do |event|
        detected_events << event
        throw(:stop) if detected_events.any? { |detected_event| detected_event.path.include?("new_file.txt") }
      end
    end

    file_event = detected_events.find { |event| event.path.include?("new_file.txt") }
    refute_nil file_event, "Expected an event for new_file.txt, got: #{detected_events.map(&:inspect)}"
    assert_equal "created", file_event.kind
  ensure
    FileUtils.rm_rf(directory)
  end

  it "accepts an array of paths" do
    directory = Dir.mktmpdir("cruise-test")
    detected_events = []

    Thread.new do
      sleep 0.5
      File.write(File.join(directory, "test.txt"), "content")
    end

    catch(:stop) do
      Cruise.watch([directory]) do |event|
        detected_events << event
        throw(:stop) if detected_events.any? { |detected_event| detected_event.path.include?("test.txt") }
      end
    end

    assert_operator detected_events.length, :>=, 1
  ensure
    FileUtils.rm_rf(directory)
  end

  it "accepts a proc callback" do
    directory = Dir.mktmpdir("cruise-test")
    detected_events = []

    Thread.new do
      sleep 0.5
      File.write(File.join(directory, "proc_test.txt"), "content")
    end

    catch(:stop) do
      Cruise.watch(directory, callback: proc { |event|
        detected_events << event
        throw(:stop) if detected_events.any? { |detected_event| detected_event.path.include?("proc_test.txt") }
      })
    end

    assert_operator detected_events.length, :>=, 1
  ensure
    FileUtils.rm_rf(directory)
  end

  it "filters events by glob pattern" do
    directory = Dir.mktmpdir("cruise-test")
    detected_events = []

    Thread.new do
      sleep 0.5
      File.write(File.join(directory, "skip.txt"), "should be filtered")
      sleep 0.3
      File.write(File.join(directory, "match.html.erb"), "should match")
    end

    catch(:stop) do
      Cruise.watch(directory, glob: "**/*.html.erb") do |event|
        detected_events << event
        throw(:stop) if detected_events.any? { |detected_event| detected_event.path.include?("match.html.erb") }
      end
    end

    txt_event = detected_events.find { |event| event.path.include?("skip.txt") }
    assert_nil txt_event, "Expected skip.txt to be filtered out"

    erb_event = detected_events.find { |event| event.path.include?("match.html.erb") }
    refute_nil erb_event, "Expected match.html.erb to pass through"
  ensure
    FileUtils.rm_rf(directory)
  end

  it "accepts multiple glob patterns" do
    directory = Dir.mktmpdir("cruise-test")
    detected_events = []

    Thread.new do
      sleep 0.5
      File.write(File.join(directory, "test.html"), "html file")
      sleep 0.3
      File.write(File.join(directory, "test.erb"), "erb file")
    end

    catch(:stop) do
      Cruise.watch(directory, glob: ["**/*.html", "**/*.erb"]) do |event|
        detected_events << event
        throw(:stop) if detected_events.any? { |detected_event| detected_event.path.include?("test.erb") }
      end
    end

    assert_operator detected_events.length, :>=, 1
  ensure
    FileUtils.rm_rf(directory)
  end

  it "exposes event path and kind" do
    directory = Dir.mktmpdir("cruise-test")
    detected_event = nil

    Thread.new do
      sleep 0.5
      File.write(File.join(directory, "test.txt"), "content")
    end

    catch(:stop) do
      Cruise.watch(directory) do |event|
        detected_event = event
        throw(:stop)
      end
    end

    refute_nil detected_event
    assert_respond_to detected_event, :path
    assert_respond_to detected_event, :kind
    assert_respond_to detected_event, :inspect
    assert_respond_to detected_event, :to_s
  ensure
    FileUtils.rm_rf(directory)
  end
end

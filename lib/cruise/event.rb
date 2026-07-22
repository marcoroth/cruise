# frozen_string_literal: true

module Cruise
  class Event
    attr_reader :path, :kind

    def initialize(path, kind)
      @path = path
      @kind = kind
    end

    def inspect
      "#<Cruise::Event kind=#{kind.inspect} path=#{path.inspect}>"
    end

    def to_s
      "#{kind}: #{path}"
    end
  end
end

# frozen_string_literal: true

module Cruise
  class Event
    attr_reader :path #: String
    attr_reader :kind #: String

    #: (String path, String kind) -> void
    def initialize(path, kind)
      @path = path
      @kind = kind
    end

    #: () -> String
    def inspect
      "#<Cruise::Event kind=#{kind.inspect} path=#{path.inspect}>"
    end

    #: () -> String
    def to_s
      "#{kind}: #{path}"
    end
  end
end

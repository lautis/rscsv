require_relative 'rscsv/rscsv'
require_relative 'rscsv/version'

module Rscsv
  Reader = RscsvReader

  class Reader
    def self.each(input, &block)
      each_internal(input, &block)
    rescue StopIteration
      nil
    end
  end

  Writer = RscsvWriter
end

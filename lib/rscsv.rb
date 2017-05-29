require 'helix_runtime'
require 'rscsv/native'
require 'rscsv/version'

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

require 'spec_helper'
require 'csv'

RSpec.describe Rscsv::Reader do
  let(:data) do
    <<~CSV
      city,country,pop
      Boston,United States,4628910
      Concord,United States,42695
    CSV
  end

  let(:row) { %w[1 2 3] }
  let(:output) { row.join(',') + "\n" }

  describe '.parse' do
    it 'parses CSV from string' do
      expect(Rscsv::Reader.parse(data)).to eq(CSV.parse(data))
    end
  end

  describe '.each' do
    it 'yields results' do
      expect(Rscsv::Reader.to_enum(:each, data).to_a).to eq(CSV.parse(data))
    end
  end
end

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

  describe '#pare' do
    it 'parses CSV' do
      expect(Rscsv::Reader.parse(data)).to eq(CSV.parse(data))
    end
  end
end

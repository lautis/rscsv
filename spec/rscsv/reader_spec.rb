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
      expect(Rscsv::Reader.to_enum(:each, data.each_char).to_a)
        .to eq(CSV.parse(data))
    end

    it 'handles when chunk size is bigger than buffer size' do
      a = 'a' * 128 * 1024
      b = 'b' * 128 * 1024
      csv = "foo,bar\n#{a},#{b}\n"
      expect(Rscsv::Reader.to_enum(:each, [csv].each).to_a)
        .to eq(CSV.parse(csv))
    end

    it 're-throws exceptions results' do
      10.times do
        expect do
          Rscsv::Reader.each(data.each_char) do |part|
            raise "error"
          end
        end.to raise_error("error")
      end
    end
  end
end

require "spec_helper"

RSpec.describe Rscsv::Writer do
  let(:row) { %w[1 2 3] }
  let(:output) { row.join(',') + "\n" }

  describe '.generate_line' do
    it 'generates csv' do
      expect(Rscsv::Writer.generate_line(row)).to eq(output)
    end
  end

  describe '.generate_lines' do
    it 'generates csv' do
      expect(Rscsv::Writer.generate_lines([row, row])).to eq(output * 2)
    end
  end
end

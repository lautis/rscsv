require "spec_helper"

RSpec.describe Rscsv::Writer do
  it "exists" do
    expect(Rscsv::Writer).to be_a(Class)
  end

  describe '.generate_line' do
    it 'generates csv' do
      row = ['1', '2', '3']
      expect(Rscsv::Writer.generate_line(row)).to eq(row.join(',') + "\n")
    end
  end
end

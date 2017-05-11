# Rscsv

Fast CSV using Rust extensions. Currently writing CSV is implemented.

## Installation

This gem requires Rust (~> 1.17) and Cargo to be installed. With those
requirements fulfilled, rscsv can be installed like any other gem:

```
gem install rscsv
```

## Usage

```ruby
require 'rscsv'

Rscsv::Writer.generate_lines([['1', '2', '3'], ['3', '4', '5']])
# => 1,2,3\n4,5,6\n
Rscsv::Writer.generate_line(['1', '2', '3'])
# => 1,2,3\n
```

This is 3x faster than using native Ruby `CSV.generate`.

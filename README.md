# Rscsv

A Rust extension to write CSV faster.

## Usage

```ruby
Rscsv::Writer.generate_lines([['1', '2', '3'], ['3', '4', '5']])
# => 1,2,3\n4,5,6\n
Rscsv::Writer.generate_line(['1', '2', '3'])
# => 1,2,3\n
```

This is 3x faster than using native Ruby `CSV.generate`.

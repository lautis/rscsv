if !system('cargo --version') || !system('rustc --version')
  raise 'You have to install Rust with Cargo (https://www.rust-lang.org/)'
end

require 'rake'

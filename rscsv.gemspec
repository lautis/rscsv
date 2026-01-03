lib = File.expand_path('../lib', __FILE__)
$LOAD_PATH.unshift(lib) unless $LOAD_PATH.include?(lib)
require 'rscsv/version'

Gem::Specification.new do |spec|
  spec.name = 'rscsv'
  spec.version = Rscsv::VERSION
  spec.authors = ['Ville Lautanala']
  spec.email = ['lautis@gmail.com']

  spec.summary = 'Rust-powered CSV'
  spec.description = 'Fast CSV using Rust extensions.'
  spec.homepage = 'https://github.com/lautis/rscsv'
  spec.license = 'MIT'

  spec.files = Dir[
    'lib/**/*.rb',
    'ext/**/*.{rs,rb,toml}',
    'Cargo.toml',
    'Cargo.lock',
    'LICENSE.txt',
    'README.md'
  ]
  spec.extensions = ['ext/rscsv/extconf.rb']
  spec.require_paths = ['lib']

  spec.required_ruby_version = '>= 3.0'

  spec.add_dependency 'rb_sys', '~> 0.9'

  spec.add_development_dependency 'bundler', '>= 2.0'
  spec.add_development_dependency 'rake', '>= 13.0'
  spec.add_development_dependency 'rake-compiler', '~> 1.2'
  spec.add_development_dependency 'rake-compiler-dock', '~> 1.5'
  spec.add_development_dependency 'rspec', '~> 3.0'
  spec.add_development_dependency 'csv', '>= 3.0'
  spec.add_development_dependency 'benchmark-ips', '~> 2.7'
end

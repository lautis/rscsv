lib = File.expand_path('../lib', __FILE__)
$LOAD_PATH.unshift(lib) unless $LOAD_PATH.include?(lib)
require 'rscsv/version'

ruby_sources = Dir['{lib/**/*,[A-Z]*}'] - Dir['Cargo.*', 'Gemfile.lock']
rust_sources = Dir['{src/**/*,ext/**/*,Cargo.*}']
native_bundle = Dir['lib/rscsv/native.bundle', 'lib/rscsv/native.so']

Gem::Specification.new do |spec|
  spec.name = 'rscsv'
  spec.version = Rscsv::VERSION
  spec.authors = ['Ville Lautanala']
  spec.email = ['lautis@gmail.com']

  spec.summary = 'Rust-powered CSV'
  spec.description = 'Fast CSV using Rust extensions.'
  spec.homepage = 'https://github.com/lautis/rscsv'
  spec.license = 'MIT'

  if ENV['NATIVE_BUNDLE']
    spec.platform = Gem::Platform.local
    spec.files = ruby_sources
  else
    spec.files = ruby_sources + rust_sources - native_bundle
    spec.extensions = Dir['ext/extconf.rb']
  end

  spec.bindir = 'exe'
  spec.executables = spec.files.grep(%r{^exe/}) { |f| File.basename(f) }
  spec.require_paths = ['lib']

  spec.add_dependency 'helix_runtime', '0.7.5'
  spec.add_development_dependency 'bundler', '>= 1.14'
  spec.add_development_dependency 'rake', '>= 10.0'
  spec.add_development_dependency 'rspec', '~> 3.0'
  spec.add_development_dependency 'benchmark-ips', '~> 2.7'
end

require 'bundler/gem_tasks'
require 'rspec/core/rake_task'
require 'rake/extensiontask'
require 'rb_sys'

RSpec::Core::RakeTask.new(:spec)

GEMSPEC = Gem::Specification.load('rscsv.gemspec')

Rake::ExtensionTask.new('rscsv', GEMSPEC) do |ext|
  ext.lib_dir = 'lib/rscsv'
  ext.source_pattern = '*.{rs,toml}'
  ext.cross_compile = true
  ext.cross_platform = %w[
    x86_64-linux
    x86_64-linux-musl
    aarch64-linux
    aarch64-linux-musl
    x86_64-darwin
    arm64-darwin
    x64-mingw-ucrt
    x64-mingw32
  ]
end

task spec: :compile
task default: [:compile, :spec]

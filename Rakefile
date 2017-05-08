require 'bundler/gem_tasks'
require 'bundler/setup'
require 'rspec/core/rake_task'
require 'helix_runtime/build_task'
HelixRuntime::BuildTask.new('rscsv')

RSpec::Core::RakeTask.new(:spec)

task :spec => :build
task :default => :spec

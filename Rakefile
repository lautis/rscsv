require 'bundler/gem_tasks'
require 'bundler/setup'
require 'rspec/core/rake_task'
import 'lib/tasks/helix_runtime.rake'

RSpec::Core::RakeTask.new(:spec)

task :spec => :build
task :default => :spec

require 'bundler/gem_tasks'
require 'bundler/setup'
require 'rspec/core/rake_task'
require 'helix_runtime/build_task'

RSpec::Core::RakeTask.new(:spec)

helix_tasks = HelixRuntime::BuildTask.new('rscsv')

# Monkey-patch project name as directory name varies during installation
module HelixRuntime
  class Project
    def name
      'rscsv'
    end
  end
end

namespace :helix do
  desc 'Build rscsv'
  task build: ['helix:pre_build', 'helix:check_path'] do
    helix_tasks.project.build
  end
end

task :spec => :build
task :default => :spec

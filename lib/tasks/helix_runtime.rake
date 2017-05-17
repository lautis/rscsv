require 'helix_runtime/build_task'

HelixRuntime::BuildTask.new('rscsv')

# Monkey-patch project name as directory name varies during installation
module HelixRuntime
  class Project
    def name
      'rscsv'
    end
  end
end

task :default => :build

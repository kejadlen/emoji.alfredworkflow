require "alphred/tasks"

task :extract_images do
  path = File.expand_path("../images", __FILE__)
  sh "gemoji extract #{path}"
end
task "alphred:package" => :extract_images

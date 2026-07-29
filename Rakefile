abort("Expected Ruby 3.4+, but got #{RUBY_VERSION}.") if RUBY_VERSION < '3.4.0'

ASIMOV_SUBCOMMANDS = %w[ask fetch handle list module read snap snapshot]

task default: %w[codegen]

desc "Generate .config/readmer/*.sh-session files"
task codegen: %w[.config/readmer/asimov.sh-session] +
  ASIMOV_SUBCOMMANDS.map { ".config/readmer/asimov-#{it}.sh-session" }.to_a

([nil] + ASIMOV_SUBCOMMANDS).each do |subcommand|
  command = subcommand ? "asimov #{subcommand} --help" : "asimov"
  filename = command.delete_suffix(' --help').gsub(' ', '-')
  desc "Generate .config/readmer/#{filename}.sh-session"
  file ".config/readmer/#{filename}.sh-session" do |t|
    File.open(t.name, 'w') do |f|
      f.puts "$ #{command}"
      #f.puts `#{command} 2>&1` # FIXME
    end
  end
end

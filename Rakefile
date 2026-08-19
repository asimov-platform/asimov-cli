abort("Expected Ruby 3.4+, but got #{RUBY_VERSION}.") if RUBY_VERSION < '3.4.0'

ASIMOV_SUBCOMMANDS = %w[module proxy source]

task default: %w[codegen]

desc "Generate etc/readmer/*.sh-session files"
task codegen: %w[etc/readmer/asimov.sh-session] +
  ASIMOV_SUBCOMMANDS.map { "etc/readmer/asimov-#{it}.sh-session" }.to_a

([nil] + ASIMOV_SUBCOMMANDS).each do |subcommand|
  command = subcommand ? "asimov #{subcommand} --help" : "asimov"
  filename = command.delete_suffix(' --help').gsub(' ', '-')
  desc "Generate etc/readmer/#{filename}.sh-session"
  file "etc/readmer/#{filename}.sh-session" do |t|
    File.open(t.name, 'w') do |f|
      f.puts "$ #{command}"
      #f.puts `#{command} 2>&1` # FIXME
    end
  end
end

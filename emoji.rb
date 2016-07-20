$LOAD_PATH.unshift(File.expand_path('../vendor/bundle', __FILE__))
require 'bundler/setup'

require 'alphred'
require 'emoji'

module Emoji
  class Character
    def code
      ":#{name}:"
    end

    def matches(query)
      !aliases.grep(query).empty? || !tags.grep(query).empty?
    end

    def to_item
      alts = aliases + tags
      alts.delete(name)
      Alphred::Item.new(
        title: name,
        uid: name,
        subtitle: alts.join(', '),
        arg: raw,
        icon: File.join(Emoji.images_path, 'emoji', image_filename),
        mods: {
          ctrl: { arg: code, subtitle: "Copy #{code} to pasteboard" }
        },
      )
    end
  end
end

if __FILE__ == $0
  query = Regexp.new(ARGV.shift)
  emojis = Emoji.all.select {|emoji| emoji.matches(query) }
  puts Alphred::Items[*emojis.map(&:to_item)].to_json
end

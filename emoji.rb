$LOAD_PATH.unshift(File.expand_path("../vendor/bundle", __FILE__))
require "bundler/setup"

require "alphred"
require "emoji"

module Emoji
  class Character
    def code
      ":#{self.name}:"
    end

    def matches(query)
      !self.aliases.grep(query).empty? || !self.tags.grep(query).empty?
    end

    def to_item
      etc = self.aliases + self.tags
      etc.delete(self.name)
      etc = etc.join(", ")
      Alphred::Item.new(
        title: self.name,
        subtitle: etc,
        arg: JSON.dump(unicode: self.raw, code: self.code),
        icon: File.join(Emoji.images_path, "emoji", self.image_filename),
        mods: { ctrl: etc },
      )
    end
  end
end

if __FILE__ == $0
  query = Regexp.new(ARGV.shift)
  emojis = Emoji.all.select {|emoji| emoji.matches(query) }
  puts Alphred::Items.new(*emojis.map(&:to_item)).to_xml
end

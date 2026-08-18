#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

abort "usage: validate_draft.rb <draft.yaml> <Format> [expected-count]" unless (2..3).cover?(ARGV.length)

path, expected_format, expected_count = ARGV
document = YAML.load_file(path)

abort "draft must be a YAML mapping" unless document.is_a?(Hash)
abort "unexpected top-level fields" unless document.keys == %w[format date archetypes]
abort "format mismatch: expected #{expected_format.inspect}" unless document["format"] == expected_format
abort "date must be a quoted YYYY-MM-DD string" unless document["date"].is_a?(String) && document["date"].match?(/\A\d{4}-\d{2}-\d{2}\z/)

archetypes = document["archetypes"]
abort "archetypes must be a non-empty array" unless archetypes.is_a?(Array) && !archetypes.empty?
abort "expected #{expected_count} archetypes" if expected_count && archetypes.length != Integer(expected_count, 10)

names = []
constraint_count = 0

archetypes.each do |archetype|
  abort "archetype must be a mapping" unless archetype.is_a?(Hash)
  unknown = archetype.keys - %w[name signatureCards strictMode]
  abort "unknown archetype fields: #{unknown.join(', ')}" unless unknown.empty?

  name = archetype["name"]
  abort "archetype name must be a non-empty string" unless name.is_a?(String) && !name.empty?
  names << name

  if archetype.key?("strictMode") && ![true, false].include?(archetype["strictMode"])
    abort "strictMode must be boolean for #{name}"
  end

  cards = archetype["signatureCards"]
  abort "signatureCards must be non-empty for #{name}" unless cards.is_a?(Array) && !cards.empty?

  cards.each do |card|
    abort "signature card must be a mapping for #{name}" unless card.is_a?(Hash)
    unknown_card = card.keys - %w[name minCopies exactCopies]
    abort "unknown signature-card fields for #{name}: #{unknown_card.join(', ')}" unless unknown_card.empty?
    abort "signature card name must be non-empty for #{name}" unless card["name"].is_a?(String) && !card["name"].empty?
    abort "signature card needs minCopies or exactCopies for #{name}" unless card.key?("minCopies") || card.key?("exactCopies")

    if card.key?("minCopies")
      value = card["minCopies"]
      abort "minCopies must be 1..4 for #{name}" unless value.is_a?(Integer) && (1..4).cover?(value)
    end
    if card.key?("exactCopies")
      value = card["exactCopies"]
      abort "exactCopies must be 0..4 for #{name}" unless value.is_a?(Integer) && (0..4).cover?(value)
    end

    constraint_count += 1
  end
end

abort "duplicate archetype names" unless names.uniq.length == names.length

source = File.read(path)
reviewed_urls = source.scan(/^  # Reviewed deck [12]: (https:\/\/\S+)$/).flatten
expected_urls = archetypes.length * 2
abort "expected #{expected_urls} reviewed deck URLs, found #{reviewed_urls.length}" unless reviewed_urls.length == expected_urls
abort "reviewed deck URLs must be unique" unless reviewed_urls.uniq.length == reviewed_urls.length

puts "validated: #{archetypes.length} archetypes, #{constraint_count} constraints, #{reviewed_urls.length} unique reviewed URLs"

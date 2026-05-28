class Caffeine < Formula
  desc "Keep your Mac awake — CLI + live menu bar countdown"
  homepage "https://github.com/juliocanizalez/caffeine"
  url "https://github.com/juliocanizalez/caffeine/archive/refs/tags/v0.1.0.tar.gz"
  # Run this to get the sha256 after creating the GitHub release:
  #   curl -sL https://github.com/juliocanizalez/caffeine/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
  sha256 "REPLACE_WITH_SHASUM_OUTPUT"
  license "MIT"
  head "https://github.com/juliocanizalez/caffeine.git", branch: "main"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Keep your Mac awake", shell_output("#{bin}/caffeine --help")
  end
end

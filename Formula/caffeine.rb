class Caffeine < Formula
  desc "Keep your Mac awake — CLI + live menu bar countdown"
  homepage "https://github.com/juliocanizalez/caffeine"
  version "0.1.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/juliocanizalez/caffeine/releases/download/v#{version}/caffeine-aarch64-apple-darwin.tar.gz"
      sha256 "b222de4d7dbd6e987f0bd48e5db06ca267e803916372020c9ea9f126b038bf78"
    end
    on_intel do
      url "https://github.com/juliocanizalez/caffeine/releases/download/v#{version}/caffeine-x86_64-apple-darwin.tar.gz"
      sha256 "b5f865bb016824eebe195a2f17b1619d7a7766be94b77cd522f01261201d662b"
    end
  end

  def install
    bin.install "caffeine"
  end

  test do
    assert_match "Keep your Mac awake", shell_output("#{bin}/caffeine --help")
  end
end

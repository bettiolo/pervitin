class Pervetin < Formula
  desc "Keeps a Mac awake on two stimulants, with an animated patient chart"
  homepage "https://github.com/bettiolo/pervetin"
  license "MIT"
  head "https://github.com/bettiolo/pervetin.git", branch: "main"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "keep the Mac awake", shell_output("#{bin}/pervetin help")
  end
end

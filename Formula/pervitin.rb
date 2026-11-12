class Pervitin < Formula
  desc "Keeps a Mac awake on two stimulants, with an animated patient chart"
  homepage "https://github.com/bettiolo/pervitin"
  license "MIT"
  head "https://github.com/bettiolo/pervitin.git", branch: "main"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "keep the Mac awake", shell_output("#{bin}/pervitin help")
  end
end

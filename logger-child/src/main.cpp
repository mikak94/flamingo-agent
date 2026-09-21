// logger-child: appends argv[1] to stdout and to the log file (argv[2]), then exits.

#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>

namespace {

constexpr int kOk = 0;
constexpr int kLogFileUnwritable = 1;
constexpr int kUsageError = 2;

bool append_line(const std::filesystem::path& path, const std::string& line) {
    std::error_code ec;
    if (!std::filesystem::exists(path, ec)) {
        std::cerr << "logger-child: " << path
                  << " isn't there, creating it (without the service's ACL)\n";
        if (path.has_parent_path()) {
            std::filesystem::create_directories(path.parent_path(), ec);
        }
    }

    std::ofstream stream(path, std::ios::out | std::ios::app);
    if (!stream.is_open()) {
        std::cerr << "logger-child: can't open " << path << "\n";
        return false;
    }
    stream << line << '\n';
    stream.flush();
    if (!stream.good()) {
        std::cerr << "logger-child: write to " << path << " failed\n";
        return false;
    }
    return true;
}

}  // namespace

int main(int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: logger-child <metrics> <log-file>\n"
                     "exits 0 ok, 1 can't write the log, 2 bad usage\n";
        return kUsageError;
    }

    const std::string metrics = argv[1];
    const std::filesystem::path log_file = argv[2];

    std::cout << metrics << '\n';
    std::cout.flush();

    return append_line(log_file, metrics) ? kOk : kLogFileUnwritable;
}

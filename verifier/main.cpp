#include <iostream>
#include <boost/program_options.hpp>
#include <thread>
#include <boost/algorithm/string/split.hpp>
#include <boost/algorithm/string/classification.hpp>
#include <boost/algorithm/string/trim.hpp>

#include <csignal>

#include "ebpf_verifier.hpp"
#include "platform/click_platform.hpp"

#include "config.hpp"

std::vector<std::string> get_sections(const std::filesystem::path &bpf_file, const ebpf_verifier_options_t &verifier_options) {
    std::vector<raw_program> raw_programs = read_elf(bpf_file, std::string(), &verifier_options, &g_ebpf_platform_click);

    std::vector<std::string> sections;
    for (const auto &raw_program: raw_programs) {
        sections.push_back(raw_program.section_name);
    }

    return sections;
}

bool verify_section(const std::filesystem::path &bpf_file, const std::string &section_name, const ebpf_verifier_options_t &verifier_options) {
    // Read a set of raw program sections from an ELF file.
    std::vector<raw_program> raw_programs =
            read_elf(bpf_file, section_name, &verifier_options, &g_ebpf_platform_click);

    // Select the last program section.
    raw_program raw_prog = raw_programs.back();

    // Convert the raw program section to a set of instructions.
    std::variant<InstructionSeq, std::string> prog_or_error =
            unmarshal(raw_prog);
    if (std::holds_alternative<std::string>(prog_or_error)) {
        throw std::runtime_error(
                "unmarshall error at "
                + std::get<std::string>(prog_or_error));
    }

    auto &prog = std::get<InstructionSeq>(prog_or_error);

    // verify with domain: zoneCrab
    ebpf_verifier_stats_t verifier_stats{};

    clock_t begin = clock();
    const auto result = ebpf_verify_program(std::cout, prog, raw_prog.info,
                                            &verifier_options, &verifier_stats);
    clock_t end = clock();

    double elapsed_ns = (double(end) * 1e9  - double(begin) * 1e9) / CLOCKS_PER_SEC;
    std::cout << "Verification took " << elapsed_ns << " ns" << std::endl;

    return result;
}

bool verify_all_sections(const std::filesystem::path &bpf_file) {
    auto verifier_options = ebpf_verifier_default_options;
    // verifier_options.print_failures = true;
    // verifier_options.print_line_info = true;

    auto successful = true;
    for (auto section_name : get_sections(bpf_file, verifier_options)) {
        std::cout << "Verifying section " << section_name << std::endl;
        auto result = verify_section(bpf_file, section_name, verifier_options);

        if (result) {
            std::cout << "Verification successful for section " << section_name << std::endl;
        } else {
            successful = false;
            std::cout << "Verification failed for section " << section_name << std::endl;
        }

        std::cout << std::endl;
    }

    return successful;
}

int main(int argc, char **argv) {
    // setup program options description
    boost::program_options::options_description cli_options(
            "ubpf-verifier terminal options");
    cli_options.add_options()
            ("file,f", boost::program_options::value<std::string>()->required(), "The bpf bytecode file to verify");

    boost::program_options::variables_map cli_options_map;

    try {
        boost::program_options::store(
                boost::program_options::parse_command_line(
                        argc, argv, cli_options),
                cli_options_map);
        boost::program_options::notify(cli_options_map);
    } catch (const std::exception &exception) {
        std::cerr << "Invalid Usage: " << exception.what() << "\n";
        std::cerr << cli_options << std::endl;
        return -1;
    }

    auto file = cli_options_map["file"].as<std::string>();
    auto success = verify_all_sections(file);

    if (success) {
        std::cout << "Verification successful for all sections" << std::endl;
    } else {
        std::cout << "Verification failed for at least one section" << std::endl;
    }

    return 0;
}

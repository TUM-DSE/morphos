#include <openssl/ec.h>
#include <openssl/ecdsa.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <openssl/sha.h>
#include <openssl/err.h>
#include <fstream>
#include <vector>

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

std::optional<raw_program>
find_main_program(const std::filesystem::path &bpf_file, const ebpf_verifier_options_t &verifier_options) {
    std::vector <raw_program> raw_programs = read_elf(bpf_file, std::string(), &verifier_options,
                                                      &g_ebpf_platform_click);

    std::vector <std::string> sections;
    for (const auto &raw_program: raw_programs) {
        std::cout << "Found function: " << raw_program.function_name << std::endl;
        if (raw_program.function_name == "main") {
            return raw_program;
        }
    }

    return {};
}

bool verify_section(const std::filesystem::path &bpf_file,
                    const ebpf_verifier_options_t &verifier_options,
                    raw_program raw_prog) {
    // Convert the raw program section to a set of instructions.
    std::variant <InstructionSeq, std::string> prog_or_error =
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

    double elapsed_ns = (double(end) * 1e9 - double(begin) * 1e9) / CLOCKS_PER_SEC;
    std::cout << "Verification took " << elapsed_ns << " ns" << std::endl;

    return result;
}

bool run_verification(const std::filesystem::path &bpf_file) {
    auto main_program = find_main_program(bpf_file, ebpf_verifier_default_options);
    if (!main_program.has_value()) {
        std::cerr << "Main program not found" << std::endl;
        return false;
    }


    auto verifier_options = ebpf_verifier_default_options;
    // verifier_options.print_failures = true;
    // verifier_options.print_line_info = true;

    std::cout << "Verifying main function" << std::endl;
    auto result = verify_section(bpf_file, verifier_options, main_program.value());

    if (result) {
        std::cout << "Verification successful" << std::endl;
    } else {
        std::cout << "Verification failed" << std::endl;
    }

    return result;
}

std::vector<unsigned char> read_file(const std::string &filePath) {
    std::ifstream file(filePath, std::ios::binary);
    if (!file) {
        throw std::runtime_error("Unable to open file");
    }
    return std::vector<unsigned char>((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
}

void handle_errors() {
    ERR_print_errors_fp(stderr);
    abort();
}

void create_signature(const std::filesystem::path &key_file,
                      const std::filesystem::path &bpf_file,
                      const std::filesystem::path &out_signature_file) {
    // open private key file
    FILE *private_key_file = fopen(key_file.c_str(), "r");
    if (!private_key_file) {
        throw new std::runtime_error("Unable to open private key file");
    }

    // read private key
    EVP_PKEY *pkey = PEM_read_PrivateKey(private_key_file, nullptr, nullptr, nullptr);
    fclose(private_key_file);
    if (!pkey) {
        throw new std::runtime_error("Unable to read private key from file");
    }

    // read the file to be signed
    std::vector<unsigned char> file_contents;
    try {
        file_contents = read_file(bpf_file);
    } catch (const std::exception &e) {
        throw new std::runtime_error("Failed to read BPF file");
    }

    // compute SHA-256 hash of the file
    unsigned char hash[SHA256_DIGEST_LENGTH];
    if (!EVP_Digest(file_contents.data(), file_contents.size(), hash, nullptr, EVP_sha256(), nullptr)) {
        throw new std::runtime_error("Failed to compute SHA-256 hash");
    }

    // create context for signing
    EVP_MD_CTX *mdctx = EVP_MD_CTX_new();
    if (!mdctx) {
        throw new std::runtime_error("Failed to create EVP_MD_CTX");
    }

    if (EVP_DigestSignInit(mdctx, nullptr, EVP_sha256(), nullptr, pkey) <= 0) {
        throw new std::runtime_error("Failed to initialize digest sign context");
    }

    // sign the hash
    size_t sig_len;
    if (EVP_DigestSign(mdctx, nullptr, &sig_len, hash, SHA256_DIGEST_LENGTH) <= 0) {
        throw new std::runtime_error("Failed to determine signature length");
    }

    unsigned char *signature = new unsigned char[sig_len];
    if (EVP_DigestSign(mdctx, signature, &sig_len, hash, SHA256_DIGEST_LENGTH) <= 0) {
        throw new std::runtime_error("Failed to sign the hash");
    }

    // write signature to file
    std::ofstream signature_file(out_signature_file, std::ios::binary);
    if (!signature_file) {
        throw new std::runtime_error("Failed to open signature file for writing");
    }

    signature_file.write(reinterpret_cast<char *>(signature), sig_len);
    signature_file.close();
}

int main(int argc, char **argv) {
    // initialize OpenSSL
    OpenSSL_add_all_algorithms();
    ERR_load_crypto_strings();

    // setup program options description
    boost::program_options::options_description cli_options(
            "ubpf-verifier terminal options");
    cli_options.add_options()
            ("file,f", boost::program_options::value<std::string>()->required(), "The BPF bytecode file to verify")
            ("key,k", boost::program_options::value<std::string>()->required(),
             "The key used for signing the BPF bytecode")
            ("out,o", boost::program_options::value<std::string>()->required(), "The output file for the signature");

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
    auto key = cli_options_map["key"].as<std::string>();
    auto out_signature_file = cli_options_map["out"].as<std::string>();

    auto success = run_verification(file);
    if (!success) {
        std::cout << "Verification failed for at least one section" << std::endl;
        return 0;
    }

    std::cout << "Verification successful for all sections" << std::endl;
    std::cout << "Generating signature..." << std::endl;

    create_signature(key, file, out_signature_file);

    std::cout << "Signature generated successfully" << std::endl;
    return 0;
}

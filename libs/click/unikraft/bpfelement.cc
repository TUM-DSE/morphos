#include <click/config.h>
#include <click/confparse.hh>
#include <click/error.hh>
#include <click/args.hh>
#include <click/standard/scheduleinfo.hh>

#include <openssl/evp.h>
#include <openssl/pem.h>
#include <openssl/sha.h>
#include <openssl/err.h>

#include <cstdio>
#include <vector>
#include <string>

#include "bpfelement.hh"

CLICK_DECLS

std::vector <uint8_t> read_file(const std::string &filename) {
    FILE *file = fopen(filename.c_str(), "rb");
    if (!file) {
        return {};
    }

    fseek(file, 0, SEEK_END);
    size_t file_size = ftell(file);
    fseek(file, 0, SEEK_SET);

    std::vector <uint8_t> buffer((file_size));
    if (fread(buffer.data(), 1, file_size, file) != file_size) {
        fclose(file);
        return {};
    }

    fclose(file);
    return buffer;
}

char write_file(const std::string &filename, const std::vector <uint8_t> &buffer) {
    FILE *file = fopen(filename.c_str(), "wb");
    if (!file) {
        return -1;
    }

    if (fwrite(buffer.data(), 1, buffer.size(), file) != buffer.size()) {
        fclose(file);
        return -1;
    }

    fclose(file);
    return 0;
}

void BPFElement::init_ubpf_vm() {
    ubpf_vm *vm = ubpf_create();
    if (vm == NULL) {
        return;
    }

    this->_bpf_map_ctx = new bpf_map_ctx();

    ubpf_toggle_bounds_check(vm, false);
    ubpf_toggle_undefined_behavior_check(vm, false);
    ubpf_register_data_relocation(vm, this->_bpf_map_ctx, do_map_relocation);
    ubpf_set_jit_code_size(vm, 128*1024); // default is 64KB

    // register bpf helpers
    ubpf_register(vm, 1, "bpf_map_lookup_elem", as_external_function_t((void *) bpf_map_lookup_elem));
    ubpf_register(vm, 2, "bpf_map_update_elem", as_external_function_t((void *) bpf_map_update_elem));
    ubpf_register(vm, 3, "bpf_map_delete_elem", as_external_function_t((void *) bpf_map_delete_elem));
    ubpf_register(vm, 5, "bpf_ktime_get_ns", as_external_function_t((void *) bpf_ktime_get_ns));
    ubpf_register(vm, 6, "bpf_trace_printk", as_external_function_t((void *) bpf_trace_printk));
    ubpf_register(vm, 7, "bpf_get_prandom_u32", as_external_function_t((void *) bpf_get_prandom_u32));
    ubpf_register(vm, 20, "unwind", as_external_function_t((void *) unwind));
    ubpf_set_unwind_function_index(vm, 20);

    this->_ubpf_vm = vm;

    register_additional_bpf_helpers();
}

void handle_jit_dump(ErrorHandler *errh, ubpf_vm *_ubpf_vm, uint64_t _bpfelement_id) {
    std::vector <uint8_t> buffer(65536);
    size_t jitted_size;
    char *error_msg = nullptr;

    if (ubpf_translate(_ubpf_vm, buffer.data(), &jitted_size, &error_msg) < 0) {
        errh->error("Error translating ubpf program: %s\n", error_msg);
        return;
    }

    char filename[50];
    sprintf(filename, "jit_dump_%lu.bin", _bpfelement_id);
    std::string jit_dump_filename = std::string(filename);

    if (write_file(jit_dump_filename, buffer) < 0) {
        errh->error("Error writing JIT dump to file\n");
        return;
    }

    uk_pr_info("Dumped JIT code to %s\n", jit_dump_filename.c_str());
}

const std::string pub_key_str = R"(
-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEK3AvjjQR+NrRqhcadKOqjkUY/OHj
RmAU5ua9+XLW8RomQQtgubMBciF2BRlzGKH6LOAxgt4RwRI6qlhVOEEegg==
-----END PUBLIC KEY-----
)";

int BPFElement::check_bpf_verification_signature(ErrorHandler *errh) {
    // Create a BIO for the public key
    BIO *bio = BIO_new_mem_buf(pub_key_str.data(), static_cast<int>(pub_key_str.size()));
    if (!bio) {
        return errh->error("Unable to create BIO for public key\n");
    }

    // Read public key from the BIO
    EVP_PKEY *pkey = PEM_read_bio_PUBKEY(bio, nullptr, nullptr, nullptr);
    BIO_free(bio);
    if (!pkey) {
        return errh->error("Failed to read public key\n");
    }

    // Read the file to be verified
    std::vector <uint8_t> file_contents = read_file(_bpf_file.c_str());
    if (file_contents.empty()) {
        EVP_PKEY_free(pkey);
        return errh->error("Failed to read file to be verified\n");
    }

    // Read the signature
    std::vector <uint8_t> signature = read_file(_signature_file.c_str());
    if (signature.empty()) {
        EVP_PKEY_free(pkey);
        return errh->error("Failed to read signature file\n");
    }

    // Compute SHA-256 hash of the file
    unsigned char hash[SHA256_DIGEST_LENGTH];
    if (!EVP_Digest(file_contents.data(), file_contents.size(), hash, nullptr, EVP_sha256(), nullptr)) {
        EVP_PKEY_free(pkey);
        return errh->error("Failed to compute SHA-256 hash\n");
    }

    // Create context for verification
    EVP_MD_CTX *mdctx = EVP_MD_CTX_new();
    if (!mdctx) {
        EVP_PKEY_free(pkey);
        return errh->error("Failed to create EVP_MD_CTX\n");
    }

    if (EVP_DigestVerifyInit(mdctx, nullptr, EVP_sha256(), nullptr, pkey) <= 0) {
        EVP_MD_CTX_free(mdctx);
        EVP_PKEY_free(pkey);
        return errh->error("Failed to initialize digest verify context\n");
    }

    // Perform verification
    if (EVP_DigestVerify(mdctx, signature.data(), signature.size(), hash, SHA256_DIGEST_LENGTH) != 1) {
        EVP_MD_CTX_free(mdctx);
        EVP_PKEY_free(pkey);
        return errh->error("Signature verification failed\n");
    }

    // Clean up
    EVP_MD_CTX_free(mdctx);
    EVP_PKEY_free(pkey);

    uk_pr_info("Signature of BPF bytecode '%s' verified successfully with signature file '%s'\n", _bpf_file.c_str(),
               _signature_file.c_str());
    return 0;
}

int BPFElement::configure(Vector <String> &conf, ErrorHandler *errh) {
    if (conf.empty()) {
        return -1;
    }

    if (Args(conf, this, errh)
                .read("ID", _bpfelement_id)
                .read("JIT", _jit)
                .read("DUMP_JIT", _dump_jit)
                .read("FILE", AnyArg(), _bpf_file)
                .read("SIGNATURE", AnyArg(), _signature_file)
                .complete() < 0) {
        return -1;
    }

    const char *filename = _bpf_file.c_str();

    bool reconfigure = _ubpf_vm != NULL;
    if (reconfigure) {
        uk_pr_info("Reconfiguring %s (ID: %lu - JIT: %d) with program %s (signature: %s)...\n", this->class_name(), _bpfelement_id, _jit,
                   filename, _signature_file.c_str());
    } else {
        uk_pr_info("Configuring %s (ID: %lu - JIT: %d) with program %s (signature: %s)...\n", this->class_name(), _bpfelement_id, _jit,
                   filename, _signature_file.c_str());
    }

    std::vector <uint8_t> buffer = read_file(filename);
    if (buffer.empty()) {
        return errh->error("Error reading file %s\n", filename);
    }

    if (!reconfigure) {
        this->init_ubpf_vm();
        if (_ubpf_vm == NULL) {
            return errh->error("Error initializing ubpf vm\n");
        }
    }

    uk_rwlock_wlock(&_lock);
    if (reconfigure) {
        ubpf_unload_code(_ubpf_vm);
    }

    char *error_msg;
    ubpf_load_elf_ex(_ubpf_vm, buffer.data(), buffer.size(), "main", &error_msg);

    if (error_msg != NULL) {
        return errh->error("Error loading ubpf program: %s\n", error_msg);
    }

#ifdef CONFIG_LIBCLICK_UBPF_VERIFY_SIGNATURE
    if (CONFIG_LIBCLICK_UBPF_VERIFY_SIGNATURE) {
        auto return_code = check_bpf_verification_signature(errh);
        if (return_code < 0) {
            return return_code;
        }
    }
#endif

    if (_jit) {
        _ubpf_jit_fn = ubpf_compile(_ubpf_vm, &error_msg);
        if (_ubpf_jit_fn == NULL) {
            return errh->error("Error compiling ubpf program: %s\n", error_msg);
        }
    }

    if (_dump_jit) {
        handle_jit_dump(errh, _ubpf_vm, _bpfelement_id);
    }

    uk_rwlock_wunlock(&_lock);

    if (reconfigure) {
        uk_pr_info("Reconfigured %s (ID: %lu - JIT: %d) with program %s\n", this->class_name(), _bpfelement_id, _jit,
                   filename);
    } else {
        uk_pr_info("Configured %s (ID: %lu - JIT: %d) with program %s\n", this->class_name(), _bpfelement_id, _jit,
                   filename);
    }

    return 0;
}

uint32_t BPFElement::exec(int port, Packet *p) {
    auto ctx = (bpfelement_md) {
            .data = (void *) p->data(),
            .data_end = (void *) p->end_data(),
            .port = port,
    };

    if (_jit) {
        return (uint32_t) _ubpf_jit_fn(&ctx, sizeof(ctx));
    } else {
        uint64_t ret;
        if (ubpf_exec(_ubpf_vm, &ctx, sizeof(ctx), &ret) != 0) {
            uk_pr_err("Error executing bpf program\n");
            return -1;
        }

        return (uint32_t) ret;
    }
}

CLICK_ENDDECLS

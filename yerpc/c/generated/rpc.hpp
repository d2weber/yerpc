#pragma once

#include <unordered_map>
#include <future>
#include <mutex>
#include <thread>
#include <atomic>
#include <functional>
#include <cstring>

extern "C" {
#include "rpc.h"
}

namespace dc {

template<typename T, typename... Args>
struct is_single_same : std::false_type {};

template<typename T, typename Arg>
struct is_single_same<T, Arg> : std::is_same<typename std::decay<Arg>::type, T> {};

// We need this to show better error messages when the deleted copy constructor is requested
template<typename Self, typename... Args>
using enable_if_not_same = typename std::enable_if<
!is_single_same<Self, Args...>::value, int>::type;

template<typename T>
struct Result {
    T result;
    std::string error_message;
    int32_t error_code;
};

template<>
struct Result<void> {
    std::string error_message;
    int32_t error_code;
};

template<typename Transport>
class Rpc {
    using Handler = std::function<void(result_t*)>;

    Transport transport_;
    std::mutex mu_;
    std::atomic<uint32_t> next_id_{1};
    std::atomic<bool> done_{false};
    std::unordered_map<uint32_t, Handler> pending_;
    std::thread reader_;

    std::function<void(std::string)> line_logger_;

    uint32_t next_id() { return next_id_++; }

    void reader_loop() {
        while (true) {
            const char* line = transport_.read();
            if (!line || done_) break;
            result_t* r = parse_response(line);
            Handler h;
            {
                std::lock_guard<std::mutex> lk(mu_);
                auto it = pending_.find(r->id);
                if (it != pending_.end()) { h = std::move(it->second); pending_.erase(it); }
            }
            if (h) h(r);
            result_unref(r);
        }
    }

    void notify(request_t* raw) {
        transport_.send(raw->json);
        request_unref(raw);
    }

    void dispatch(uint32_t id, request_t* raw, Handler h) {
        { std::lock_guard<std::mutex> lk(mu_); pending_[id] = std::move(h); }
        transport_.send(raw->json);
        request_unref(raw);
    }

public:
    Rpc(Transport t, std::function<void(std::string)> line_logger) : transport_(std::move(t)), line_logger_(line_logger) { reader_ = std::thread([this]{ reader_loop(); }); }
    ~Rpc() {
        done_ = true;
        transport_.close();
        if (reader_.joinable()) reader_.join();
        std::lock_guard<std::mutex> lk(mu_);
        for (auto& kv : pending_) kv.second(nullptr);
        pending_.clear();
    }
    Rpc(const Rpc&) = delete;
    Rpc& operator=(const Rpc&) = delete;

    std::string shout(const char* msg) {
        Result<std::string> r = shout_request(msg).get();
        if (r.error_code) { line_logger_("Error " + std::to_string(r.error_code) + " in `shout`: " + r.error_message); }
        return std::move(r.result);
    }
    std::future<Result<std::string>> shout_request(const char* msg) {
        uint32_t _id = next_id();
        auto* raw_prom = new std::promise<Result<std::string>>();
        auto fut = raw_prom->get_future();
        dispatch(_id, build_shout(_id, msg), [raw_prom](result_t* raw_res) {
            std::unique_ptr<std::promise<Result<std::string>>> prom{raw_prom};
            if (raw_res->error_code != 0) {
                prom->set_value({{}, raw_res->error_message, raw_res->error_code});
                return;
            }
            const char* res;
            if (string_from_json(raw_res->result, &res) != 0) {
                prom->set_value({{}, "parse error", -32700});
                return;
            }
            prom->set_value({res, {}, 0});
            string_unref(res);
        });
        return fut;
    }

    float add(float a, float b) {
        Result<float> r = add_request(a, b).get();
        if (r.error_code) { line_logger_("Error " + std::to_string(r.error_code) + " in `add`: " + r.error_message); }
        return std::move(r.result);
    }
    std::future<Result<float>> add_request(float a, float b) {
        uint32_t _id = next_id();
        auto* raw_prom = new std::promise<Result<float>>();
        auto fut = raw_prom->get_future();
        dispatch(_id, build_add(_id, a, b), [raw_prom](result_t* raw_res) {
            std::unique_ptr<std::promise<Result<float>>> prom{raw_prom};
            if (raw_res->error_code != 0) {
                prom->set_value({{}, raw_res->error_message, raw_res->error_code});
                return;
            }
            float res;
            if (f32_from_json(raw_res->result, &res) != 0) {
                prom->set_value({{}, "parse error", -32700});
                return;
            }
            prom->set_value({res, {}, 0});
        });
        return fut;
    }

};

} // namespace dc

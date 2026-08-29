/*
 * ping.c - ICMP echo probe using the unprivileged ping socket, with a JNI
 * entry point for the Android app. Built via CMake as libping.so; no root
 * needed on Android (net.ipv4.ping_group_range covers all groups).
 */
#include <arpa/inet.h>
#include <errno.h>
#include <jni.h>
#include <netinet/in.h>
#include <netinet/ip_icmp.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#define DEFAULT_COUNT 3
#define TIMEOUT_SEC 1
#define PAYLOAD_LEN 56
#define MAX_PKT (sizeof(struct icmphdr) + PAYLOAD_LEN)

static unsigned short g_pid;
static int g_seq = 0;
static volatile int g_stop = 0;

static void handle_alarm(int sig) { (void)sig; g_stop = 1; }

static unsigned short checksum(void *data, int len) {
    unsigned short *p = (unsigned short *)data;
    unsigned int sum = 0;
    while (len > 1) {
        sum += *p++;
        len -= 2;
    }
    if (len == 1) sum += *(unsigned char *)p;
    sum = (sum >> 16) + (sum & 0xffff);
    sum += (sum >> 16);
    return (unsigned short)~sum;
}

static double now_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return tv.tv_sec * 1000.0 + tv.tv_usec / 1000.0;
}

static int send_echo(int sock, const struct sockaddr_in *dst) {
    char pkt[MAX_PKT];
    struct icmphdr *icmp = (struct icmphdr *)pkt;
    memset(pkt, 0, sizeof(pkt));
    icmp->type = ICMP_ECHO;
    icmp->code = 0;
    icmp->checksum = 0;
    icmp->un.echo.id = htons(g_pid);
    icmp->un.echo.sequence = htons(++g_seq);
    for (int i = 0; i < PAYLOAD_LEN; i++) pkt[sizeof(struct icmphdr) + i] = (char)i;
    icmp->checksum = checksum(pkt, sizeof(pkt));
    return sendto(sock, pkt, sizeof(pkt), 0, (const struct sockaddr *)dst, sizeof(*dst));
}

static int recv_reply(int sock, double *rtt, int *reply_seq) {
    char buf[2048];
    struct sockaddr_in src;
    socklen_t slen = sizeof(src);
    struct timeval start;
    gettimeofday(&start, NULL);
    ssize_t n = recvfrom(sock, buf, sizeof(buf), 0, (struct sockaddr *)&src, &slen);
    if (n < 0) return -1;
    /* Datagram ping sockets deliver the ICMP header at the start of the
     * buffer (no IP header, unlike raw sockets). The kernel assigns the
     * echo id (ignoring the one we set), so only match echo replies by
     * packet type here. */
    struct icmphdr *icmp = (struct icmphdr *)buf;
    if (icmp->type == ICMP_ECHOREPLY) {
        struct timeval end;
        gettimeofday(&end, NULL);
        double ms = (end.tv_sec - start.tv_sec) * 1000.0
                  + (end.tv_usec - start.tv_usec) / 1000.0;
        *rtt = ms;
        *reply_seq = ntohs(icmp->un.echo.sequence);
        return 0;
    }
    return 1; /* not ours; keep waiting */
}

/** Runs a ping and returns the textual report (same shape as the CLI ping). */
static char *run_ping(const char *target, int count) {
    struct sockaddr_in dst;
    memset(&dst, 0, sizeof(dst));
    dst.sin_family = AF_INET;
    if (inet_pton(AF_INET, target, &dst.sin_addr) != 1) {
        return strdup("ping: bad address\n");
    }

    int sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP);
    if (sock < 0) {
        char msg[256];
        snprintf(msg, sizeof(msg), "ping: cannot create ping socket: %s\n",
                 strerror(errno));
        return strdup(msg);
    }

    struct timeval tv;
    tv.tv_sec = TIMEOUT_SEC;
    tv.tv_usec = 0;
    setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    g_pid = (unsigned short)getpid();
    g_seq = 0;
    char ipstr[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &dst.sin_addr, ipstr, sizeof(ipstr));

    char *out = malloc(4096);
    if (!out) { close(sock); return strdup("ping: out of memory\n"); }
    out[0] = 0;
    int n = snprintf(out, 4096, "PING %s (%s): %d data bytes\n",
                     target, ipstr, PAYLOAD_LEN);

    signal(SIGALRM, handle_alarm);
    int sent = 0, received = 0;
    double rtt_min = 1e9, rtt_max = 0, rtt_sum = 0;

    /* Send all probes back-to-back (no inter-packet sleep), then collect
     * replies until the timeout. Parallel send keeps total latency close to
     * a single RTT instead of count * interval. */
    for (int i = 0; i < count && !g_stop; i++) {
        if (send_echo(sock, &dst) >= 0) {
            sent++;
        } else {
            n += snprintf(out + n, 4096 - n, "ping: sendto failed: %s\n",
                          strerror(errno));
            break;
        }
    }
    if (sent > 0) {
        alarm(TIMEOUT_SEC);
        while (!g_stop && received < sent) {
            double rtt;
            int reply_seq;
            int r = recv_reply(sock, &rtt, &reply_seq);
            if (r == 0) {
                received++;
                rtt_min = rtt < rtt_min ? rtt : rtt_min;
                rtt_max = rtt > rtt_max ? rtt : rtt_max;
                rtt_sum += rtt;
                n += snprintf(out + n, 4096 - n,
                              "64 bytes from %s: icmp_seq=%d time=%.1f ms\n",
                              ipstr, reply_seq, rtt);
            } else if (r < 0) {
                if (errno == EAGAIN || errno == EWOULDBLOCK) {
                    for (int m = received; m < sent; m++) {
                        n += snprintf(out + n, 4096 - n,
                                      "Request timeout for icmp_seq=%d\n", m + 1);
                    }
                } else if (errno != EINTR) {
                    n += snprintf(out + n, 4096 - n,
                                  "ping: recvfrom failed: %s\n", strerror(errno));
                }
                break;
            }
        }
        alarm(0);
        g_stop = 0;
    }
    close(sock);

    n += snprintf(out + n, 4096 - n, "\n--- %s ping statistics ---\n", target);
    n += snprintf(out + n, 4096 - n,
                  "%d packets transmitted, %d received, %.0f%% packet loss\n",
                  sent, received, sent ? (100.0 * (sent - received) / sent) : 0.0);
    if (received > 0) {
        n += snprintf(out + n, 4096 - n, "rtt min/avg/max = %.1f/%.1f/%.1f ms\n",
                      rtt_min, rtt_sum / received, rtt_max);
    }
    return out;
}

JNIEXPORT jstring JNICALL
Java_com_hyperscope_android_data_NativePing_pingNative(JNIEnv *env, jobject thiz,
                                                       jstring jhost, jint jcount) {
    (void)thiz;
    const char *host = (*env)->GetStringUTFChars(env, jhost, NULL);
    if (!host) return (*env)->NewStringUTF(env, "ping: bad host\n");
    char *report = run_ping(host, (int)jcount);
    (*env)->ReleaseStringUTFChars(env, jhost, host);
    jstring result = (*env)->NewStringUTF(env, report);
    free(report);
    return result;
}

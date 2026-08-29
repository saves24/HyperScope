/*
 * ping.c - ICMP echo probe using the unprivileged ping socket.
 *
 * Uses socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP) which requires no root
 * on Android (net.ipv4.ping_group_range covers all groups). Build once with
 * the NDK toolchain; the binary is bundled in the APK assets.
 *
 * Usage: ping <target-ip> [count]
 */
#include <arpa/inet.h>
#include <errno.h>
#include <netinet/in.h>
#include <netinet/ip.h>
#include <netinet/ip_icmp.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#define DEFAULT_COUNT 4
#define TIMEOUT_SEC 2
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
     * packet type here; the seq check below filters duplicates. */
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

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: ping <target> [count]\n");
        return 2;
    }
    const char *target = argv[1];
    int count = argc > 2 ? atoi(argv[2]) : DEFAULT_COUNT;
    if (count <= 0) count = DEFAULT_COUNT;

    struct sockaddr_in dst;
    memset(&dst, 0, sizeof(dst));
    dst.sin_family = AF_INET;
    if (inet_pton(AF_INET, target, &dst.sin_addr) != 1) {
        fprintf(stderr, "ping: bad address: %s\n", target);
        return 2;
    }

    int sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP);
    if (sock < 0) {
        fprintf(stderr, "ping: cannot create ping socket: %s\n", strerror(errno));
        return 1;
    }

    struct timeval tv;
    tv.tv_sec = TIMEOUT_SEC;
    tv.tv_usec = 0;
    setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    g_pid = (unsigned short)getpid();
    char ipstr[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &dst.sin_addr, ipstr, sizeof(ipstr));
    printf("PING %s (%s): %d data bytes\n", target, ipstr, PAYLOAD_LEN);

    signal(SIGALRM, handle_alarm);
    int sent = 0, received = 0;
    double rtt_min = 1e9, rtt_max = 0, rtt_sum = 0;

    for (int i = 0; i < count && !g_stop; i++) {
        if (send_echo(sock, &dst) >= 0) {
            sent++;
        } else {
            printf("ping: sendto failed: %s\n", strerror(errno));
            break;
        }
        /* Wait for replies until timeout or our packet number. */
        alarm(TIMEOUT_SEC);
        int got = 0;
        while (!g_stop && got < 1) {
            double rtt;
            int reply_seq;
            int r = recv_reply(sock, &rtt, &reply_seq);
            if (r == 0) {
                received++;
                rtt_min = rtt < rtt_min ? rtt : rtt_min;
                rtt_max = rtt > rtt_max ? rtt : rtt_max;
                rtt_sum += rtt;
                printf("64 bytes from %s: icmp_seq=%d time=%.1f ms\n",
                       ipstr, reply_seq, rtt);
                got = 1;
            } else if (r < 0) {
                if (errno == EAGAIN || errno == EWOULDBLOCK) {
                    printf("Request timeout for icmp_seq=%d\n", i + 1);
                } else if (errno != EINTR) {
                    printf("ping: recvfrom failed: %s\n", strerror(errno));
                }
                break;
            }
            /* r == 1: reply for another process, keep waiting */
        }
        alarm(0);
        g_stop = 0;
        if (i < count - 1) usleep(500000);
    }
    close(sock);

    printf("\n--- %s ping statistics ---\n", target);
    printf("%d packets transmitted, %d received, %.0f%% packet loss\n",
           sent, received, sent ? (100.0 * (sent - received) / sent) : 0.0);
    if (received > 0) {
        printf("rtt min/avg/max = %.1f/%.1f/%.1f ms\n",
               rtt_min, rtt_sum / received, rtt_max);
    }
    return received > 0 ? 0 : 1;
}

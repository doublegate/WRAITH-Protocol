/*
 * XDP packet filter for WRAITH Protocol
 *
 * Filters UDP packets destined for WRAITH port range (40000-50000)
 * and redirects them to AF_XDP sockets for zero-copy processing.
 *
 * Target performance: >24M pps single-core packet processing
 *
 * Compile with:
 *   clang -O2 -target bpf -c xdp_filter.c -o xdp_filter.o
 *
 * Load with:
 *   ip link set dev <interface> xdp obj xdp_filter.o sec xdp
 */

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/ipv6.h>
#include <linux/udp.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

/* WRAITH port range */
#define WRAITH_PORT_MIN 40000
#define WRAITH_PORT_MAX 50000

/* Maximum number of AF_XDP sockets */
#define MAX_SOCKETS 64

/*
 * Map for AF_XDP socket file descriptors
 * Key: Queue ID
 * Value: AF_XDP socket FD
 */
struct {
    __uint(type, BPF_MAP_TYPE_XSKMAP);
    __uint(key_size, sizeof(__u32));
    __uint(value_size, sizeof(__u32));
    __uint(max_entries, MAX_SOCKETS);
} xsks_map SEC(".maps");

/*
 * Per-CPU statistics map
 * Indices: 0=rx_packets, 1=rx_bytes, 2=dropped, 3=redirected
 */
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(key_size, sizeof(__u32));
    __uint(value_size, sizeof(__u64));
    __uint(max_entries, 4);
} stats_map SEC(".maps");

/* Statistic indices */
enum stat_type {
    STAT_RX_PACKETS = 0,
    STAT_RX_BYTES = 1,
    STAT_DROPPED = 2,
    STAT_REDIRECTED = 3,
};

/*
 * Update statistics counter
 */
static __always_inline void update_stat(__u32 type, __u64 delta)
{
    __u64 *value = bpf_map_lookup_elem(&stats_map, &type);
    if (value)
        __sync_fetch_and_add(value, delta);
}

/*
 * Parse Ethernet header
 * Returns 0 on success, -1 on error
 */
static __always_inline int parse_ethhdr(struct xdp_md *ctx,
                                        void **data,
                                        void **data_end,
                                        struct ethhdr **ethhdr)
{
    *data = (void *)(long)ctx->data;
    *data_end = (void *)(long)ctx->data_end;

    *ethhdr = *data;
    if (*data + sizeof(struct ethhdr) > *data_end)
        return -1;

    return 0;
}

/*
 * Parse IPv4 header
 * Returns 0 on success, -1 on error
 */
static __always_inline int parse_iphdr(void *data, void *data_end,
                                       struct iphdr **iphdr)
{
    *iphdr = data + sizeof(struct ethhdr);
    if ((void *)*iphdr + sizeof(struct iphdr) > data_end)
        return -1;

    /* Only IPv4 */
    if ((*iphdr)->version != 4)
        return -1;

    /* Check for IP fragmentation */
    if ((*iphdr)->frag_off & bpf_htons(0x1FFF))
        return -1;

    return 0;
}

/*
 * Parse IPv6 header
 * Returns 0 on success, -1 on error
 */
static __always_inline int parse_ipv6hdr(void *data, void *data_end,
                                         struct ipv6hdr **ipv6hdr)
{
    *ipv6hdr = data + sizeof(struct ethhdr);
    if ((void *)*ipv6hdr + sizeof(struct ipv6hdr) > data_end)
        return -1;

    /* Only IPv6 */
    if (((*ipv6hdr)->version) != 6)
        return -1;

    return 0;
}

/*
 * Parse UDP header (IPv4)
 * Returns 0 on success, -1 on error
 */
static __always_inline int parse_udphdr_v4(void *data, void *data_end,
                                           struct iphdr *iphdr,
                                           struct udphdr **udphdr)
{
    if (iphdr->protocol != IPPROTO_UDP)
        return -1;

    *udphdr = (void *)iphdr + (iphdr->ihl * 4);
    if ((void *)*udphdr + sizeof(struct udphdr) > data_end)
        return -1;

    return 0;
}

/*
 * Parse UDP header (IPv6)
 * Returns 0 on success, -1 on error
 */
static __always_inline int parse_udphdr_v6(void *data, void *data_end,
                                           struct ipv6hdr *ipv6hdr,
                                           struct udphdr **udphdr)
{
    if (ipv6hdr->nexthdr != IPPROTO_UDP)
        return -1;

    *udphdr = (void *)ipv6hdr + sizeof(struct ipv6hdr);
    if ((void *)*udphdr + sizeof(struct udphdr) > data_end)
        return -1;

    return 0;
}

/*
 * Check if destination port is in WRAITH range
 */
static __always_inline int is_wraith_port(__u16 dport)
{
    return (dport >= WRAITH_PORT_MIN && dport <= WRAITH_PORT_MAX);
}

/*
 * Main XDP program
 *
 * Filters UDP packets for WRAITH protocol and redirects to AF_XDP sockets.
 *
 * Return values:
 *   XDP_PASS     - Pass packet to kernel network stack
 *   XDP_DROP     - Drop packet
 *   XDP_REDIRECT - Redirect to AF_XDP socket
 */
SEC("xdp")
int xdp_wraith_filter(struct xdp_md *ctx)
{
    void *data, *data_end;
    struct ethhdr *eth;
    __u16 eth_proto;
    __u16 dport;
    __u32 queue_id;
    int ret;

    /* Parse Ethernet header */
    if (parse_ethhdr(ctx, &data, &data_end, &eth) < 0)
        goto pass;

    eth_proto = bpf_ntohs(eth->h_proto);

    /* Handle IPv4 */
    if (eth_proto == ETH_P_IP) {
        struct iphdr *ip;
        struct udphdr *udp;

        /* Parse IP header */
        if (parse_iphdr(data, data_end, &ip) < 0)
            goto pass;

        /* Parse UDP header */
        if (parse_udphdr_v4(data, data_end, ip, &udp) < 0)
            goto pass;

        /* Check destination port */
        dport = bpf_ntohs(udp->dest);
        if (!is_wraith_port(dport))
            goto pass;

        /* Update statistics */
        update_stat(STAT_RX_PACKETS, 1);
        update_stat(STAT_RX_BYTES, data_end - data);

        /* Redirect to AF_XDP socket based on RX queue */
        queue_id = ctx->rx_queue_index;
        ret = bpf_redirect_map(&xsks_map, queue_id, 0);
        if (ret == XDP_REDIRECT) {
            update_stat(STAT_REDIRECTED, 1);
            return XDP_REDIRECT;
        }

        update_stat(STAT_DROPPED, 1);
        return XDP_DROP;
    }

    /* Handle IPv6 */
    if (eth_proto == ETH_P_IPV6) {
        struct ipv6hdr *ipv6;
        struct udphdr *udp;

        /* Parse IPv6 header */
        if (parse_ipv6hdr(data, data_end, &ipv6) < 0)
            goto pass;

        /* Parse UDP header */
        if (parse_udphdr_v6(data, data_end, ipv6, &udp) < 0)
            goto pass;

        /* Check destination port */
        dport = bpf_ntohs(udp->dest);
        if (!is_wraith_port(dport))
            goto pass;

        /* Update statistics */
        update_stat(STAT_RX_PACKETS, 1);
        update_stat(STAT_RX_BYTES, data_end - data);

        /* Redirect to AF_XDP socket based on RX queue */
        queue_id = ctx->rx_queue_index;
        ret = bpf_redirect_map(&xsks_map, queue_id, 0);
        if (ret == XDP_REDIRECT) {
            update_stat(STAT_REDIRECTED, 1);
            return XDP_REDIRECT;
        }

        update_stat(STAT_DROPPED, 1);
        return XDP_DROP;
    }

pass:
    /* Pass all non-WRAITH traffic to kernel */
    return XDP_PASS;
}

/* License required for BPF programs */
char _license[] SEC("license") = "GPL";

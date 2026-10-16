/* Private SDK boundary: no copied socket_fdinfo layout crosses into Rust. */
#include <libproc.h>
#include <sys/proc_info.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

struct orkworks_socket_tuple {
    uint32_t local_address;
    uint32_t peer_address;
    uint16_t local_port;
    uint16_t peer_port;
};
_Static_assert(sizeof(struct orkworks_socket_tuple) == 12, "tuple ABI");
_Static_assert(sizeof(struct proc_fdinfo) == 8, "descriptor ABI");
_Static_assert(offsetof(struct proc_fdinfo, proc_fd) == 0, "descriptor fd ABI");
_Static_assert(offsetof(struct proc_fdinfo, proc_fdtype) == 4, "descriptor kind ABI");
_Static_assert(PROX_FDTYPE_SOCKET == 2, "descriptor socket kind ABI");
_Static_assert(sizeof(struct socket_fdinfo) <= 1024 * 1024, "socket buffer bound");

/* -1 is unavailable, 0 is a non-established/non-IPv4-TCP socket, 1 is decoded. */
int orkworks_native_socket(int pid, int fd, struct orkworks_socket_tuple *out) {
    struct socket_fdinfo info;
    memset(&info, 0, sizeof(info));
    int length = proc_pidfdinfo(pid, fd, PROC_PIDFDSOCKETINFO, &info, sizeof(info));
    if (length != sizeof(info)) return -1;
    if (info.psi.soi_family != AF_INET || info.psi.soi_protocol != IPPROTO_TCP
        || info.psi.soi_type != SOCK_STREAM || info.psi.soi_kind != SOCKINFO_TCP)
        return 0;
    const struct tcp_sockinfo *tcp = &info.psi.soi_proto.pri_tcp;
    const struct in_sockinfo *ip = &tcp->tcpsi_ini;
    if (tcp->tcpsi_state != TSI_S_ESTABLISHED || !(ip->insi_vflag & INI_IPV4))
        return 0;
    if (ip->insi_lport < 0 || ip->insi_lport > UINT16_MAX
        || ip->insi_fport < 0 || ip->insi_fport > UINT16_MAX) return -1;
    out->local_address = ntohl(ip->insi_laddr.ina_46.i46a_addr4.s_addr);
    out->peer_address = ntohl(ip->insi_faddr.ina_46.i46a_addr4.s_addr);
    out->local_port = ntohs((uint16_t)ip->insi_lport);
    out->peer_port = ntohs((uint16_t)ip->insi_fport);
    return 1;
}

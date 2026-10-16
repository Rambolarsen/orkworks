/* Exercise the real SDK adapter against controlled kernel reply failures. */
#include "../src/runtime/codex_native/listener_identity.c"
#include <assert.h>
static struct socket_fdinfo reply;
static int reply_length;
int proc_pidfdinfo(int pid, int fd, int flavor, void *buffer, int size) {
    (void)pid; (void)fd;
    assert(flavor == PROC_PIDFDSOCKETINFO);
    assert(size == sizeof(reply));
    memcpy(buffer, &reply, sizeof(reply));
    return reply_length;
}
int main(void) {
    struct orkworks_socket_tuple out = {0};
    reply_length = sizeof(reply);
    reply.psi.soi_family = AF_INET; reply.psi.soi_protocol = IPPROTO_TCP;
    reply.psi.soi_type = SOCK_STREAM; reply.psi.soi_kind = SOCKINFO_TCP;
    struct tcp_sockinfo *tcp = &reply.psi.soi_proto.pri_tcp;
    tcp->tcpsi_state = TSI_S_ESTABLISHED;
    tcp->tcpsi_ini.insi_vflag = INI_IPV4;
    tcp->tcpsi_ini.insi_laddr.ina_46.i46a_addr4.s_addr = htonl(0x7f000001);
    tcp->tcpsi_ini.insi_faddr.ina_46.i46a_addr4.s_addr = htonl(0x7f000002);
    tcp->tcpsi_ini.insi_lport = htons(10001);
    tcp->tcpsi_ini.insi_fport = htons(20002);
    assert(orkworks_native_socket(42,3,&out) == 1);
    assert(out.local_address == 0x7f000001 && out.peer_address == 0x7f000002);
    assert(out.local_port == 10001 && out.peer_port == 20002);
    for (int state = TSI_S_CLOSED; state <= TSI_S_RESERVED; state++) {
        tcp->tcpsi_state = state;
        assert(orkworks_native_socket(42,3,&out) == (state == TSI_S_ESTABLISHED ? 1 : 0));
    }
    tcp->tcpsi_state = TSI_S_ESTABLISHED;
    reply.psi.soi_family = AF_INET6; assert(orkworks_native_socket(42,3,&out) == 0);
    reply.psi.soi_family = AF_INET; reply.psi.soi_protocol = IPPROTO_UDP;
    assert(orkworks_native_socket(42,3,&out) == 0);
    reply.psi.soi_protocol = IPPROTO_TCP; reply.psi.soi_kind = SOCKINFO_IN;
    assert(orkworks_native_socket(42,3,&out) == 0); reply.psi.soi_kind = SOCKINFO_TCP;
    reply_length = sizeof(reply)-1; assert(orkworks_native_socket(42,3,&out) == -1);
    reply_length = sizeof(reply)+1; assert(orkworks_native_socket(42,3,&out) == -1);
    reply_length = -1; assert(orkworks_native_socket(42,3,&out) == -1);
    reply_length = sizeof(reply); tcp->tcpsi_ini.insi_lport = 65536;
    assert(orkworks_native_socket(42,3,&out) == -1);
    tcp->tcpsi_ini.insi_lport = -1; assert(orkworks_native_socket(42,3,&out) == -1);
    return 0;
}

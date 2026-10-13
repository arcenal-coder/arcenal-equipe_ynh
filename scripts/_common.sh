#!/bin/bash

arcenal_check_binary() {
    local binary="$install_dir/bin/arcenal-equipe"
    test -f "$binary" || ynh_die "Binaire Linux manquant."
    local expected
    case "$(dpkg --print-architecture)" in
        amd64) expected="3e00" ;;
        arm64) expected="b700" ;;
        *) ynh_die "Architecture non prise en charge." ;;
    esac
    test "$(od -An -tx1 -N4 "$binary" | tr -d ' \n')" = "7f454c46" || ynh_die "Le binaire doit être un ELF Linux."
    test "$(od -An -tx1 -j18 -N2 "$binary" | tr -d ' \n')" = "$expected" || ynh_die "Architecture du binaire incorrecte."
}

arcenal_deploy_files() {
    install -d -m 750 "$install_dir/bin" "$install_dir/lib" "$install_dir/profiles" "$install_dir/config"
    if test -f ../dist/arcenal-equipe; then
        install -m 755 ../dist/arcenal-equipe "$install_dir/bin/arcenal-equipe"
    else
        ynh_setup_source --dest_dir="$install_dir/bin"
        chmod 755 "$install_dir/bin/arcenal-equipe"
    fi
    arcenal_check_binary
    install -m 644 ../profiles/*.yaml "$install_dir/profiles/"
    chown -R "root:$app" "$install_dir"
}

arcenal_configure() {
    chmod 600 "/etc/nginx/conf.d/$domain.d/$app.conf"
    chown root:root "/etc/nginx/conf.d/$domain.d/$app.conf"
    ynh_config_add_systemd
    yunohost service add "$app" --description="ARCenal Équipe native"
    ynh_systemctl --service="$app" --action=start
    ynh_systemctl --service=nginx --action=reload
}

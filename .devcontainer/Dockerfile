FROM debian:sid

ARG USERNAME=veildev
ARG WORKSPACE=/workspaces/veil
ARG UID=1000
ARG GID=1000
ARG WORKSPACE=/workspaces/veil
ARG DEBIAN_FRONTEND=noninteractive
################################## Base Devcontainer ####################################################
# Enable source, contrib and non-free repositories, upgrade packages and install basic utilities
RUN sed -i -e "s/main/main contrib non-free non-free-firmware/g" -e "s/Types: deb/Types: deb deb-src/g" \
    /etc/apt/sources.list.d/debian.sources && apt update && apt upgrade -y

RUN apt install -y apt-transport-https apt-utils ca-certificates curl dialog \
    git git-lfs gnupg2 htop iproute2 jq less libc6 libgssapi-krb5-2 \
    libkrb5-3 libstdc++6 locales lsb-release lsof man-db manpages \
    manpages-dev nano ncdu net-tools openssh-client procps psmisc \
    rsync strace sudo unzip vim-tiny wget zip zlib1g zsh build-essential \
    protobuf-compiler

########################################## User #########################################################
# Create user
RUN groupadd --gid $GID $USERNAME && \ 
    useradd -m -u $UID -g $GID -s /usr/bin/zsh $USERNAME && \ 
    usermod -aG video $USERNAME && \ 
    echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" >/etc/sudoers.d/$USERNAME && \ 
    chmod 0440 /etc/sudoers.d/$USERNAME

# Create XDG_RUNTIME_DIR
RUN mkdir -p /run/user/$UID && \ 
    chown $USERNAME:$USERNAME /run/user/$UID && \ 
    chmod 700 /run/user/$UID

USER $USERNAME
WORKDIR $WORKSPACE
SHELL ["/usr/bin/zsh", "-c"]
ENV SHELL=/usr/bin/zsh
ENV HOME=/home/$USERNAME
ENV PATH=/home/$USERNAME/.local/bin:$PATH
##################################### Dev Utilities #####################################################
# oh-my-zsh (should go first because it overwrites the .zshrc file)
RUN export ZSH_PLUGINS_DIR=$HOME/.oh-my-zsh/custom/plugins && \
    zsh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" && \ 
    git clone https://github.com/zsh-users/zsh-syntax-highlighting.git $ZSH_PLUGINS_DIR/zsh-syntax-highlighting && \ 
    git clone https://github.com/zsh-users/zsh-completions $ZSH_PLUGINS_DIR/zsh-completions && \ 
    git clone https://github.com/ptavares/zsh-direnv.git $ZSH_PLUGINS_DIR/zsh-direnv && \ 
    sed -i "s/plugins=(git)/plugins=(git zsh-syntax-highlighting zsh-completions zsh-direnv)/g" ~/.zshrc

# direnv
RUN mkdir -p $HOME/.config/direnv
COPY .devcontainer/direnv.toml $HOME/.config/direnv/direnv.toml
RUN export BIN_PATH=$HOME/.local/bin && \
    mkdir -p $BIN_PATH && \
    export bin_path=$BIN_PATH && \
    zsh -c "$(curl -fsSL https://direnv.net/install.sh)"
########################################## Build dependencies ###########################################
# rust
RUN zsh -c "$(curl -fsSL https://sh.rustup.rs)" -- -y && \
    echo "source $HOME/.cargo/env" >> $HOME/.zshrc
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | zsh && \
    cargo binstall watchexec-cli -y

# node
ENV PNPM_STORE=$WORKSPACE/.pnpm-store
RUN bash -c "$(curl -fsSL https://get.pnpm.io/install.sh)"
RUN source $HOME/.zshrc && pnpm env use --global latest
RUN echo "alias p='pnpm'" >> $HOME/.zshrc

# protobuf
RUN cargo install protoc-gen-prost protoc-gen-prost-serde protoc-gen-tonic protoc-gen-prost-crate
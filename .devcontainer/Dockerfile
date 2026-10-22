FROM debian:sid

ARG USERNAME=veildev
ARG UID=1000
ARG GID=1000
ARG DEBIAN_FRONTEND=noninteractive
################################## Base Devcontainer ####################################################
# Enable package repositories, upgrade packages and install basic utilities
RUN sed -i -e "s/main/main contrib non-free non-free-firmware/g" -e "s/Types: deb/Types: deb deb-src/g" \
    /etc/apt/sources.list.d/debian.sources && dpkg --add-architecture i386

RUN apt update && apt upgrade -y && apt install -y apt-transport-https apt-utils \
        ca-certificates curl dialog git git-lfs gnupg2 htop iproute2 jq less libc6 \
        libgssapi-krb5-2 libkrb5-3 libstdc++6 locales lsb-release lsof man-db manpages \
        manpages-dev nano ncdu net-tools openssh-client procps psmisc rsync strace sudo \
        unzip vim-tiny wget zip zlib1g zsh build-essential protobuf-compiler libxml2

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
    chmod 700 /run/user/$UID && \
    echo "export XDG_RUNTIME_DIR=/run/user/$UID" >> /etc/profile

USER $USERNAME
SHELL ["/usr/bin/zsh", "-c"]
ENV HOME=/home/$USERNAME
ENV PATH=/home/$USERNAME/.local/bin:$PATH
ENV SHELL=/usr/bin/zsh
WORKDIR $HOME/veil

##################################### Dev Utilities #####################################################
# oh-my-zsh (should go first because it overwrites the .zshrc file)
RUN export ZSH_PLUGINS_DIR=$HOME/.oh-my-zsh/custom/plugins && \
    curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh | bash && \ 
    git clone https://github.com/zsh-users/zsh-syntax-highlighting.git $ZSH_PLUGINS_DIR/zsh-syntax-highlighting && \ 
    git clone https://github.com/zsh-users/zsh-completions $ZSH_PLUGINS_DIR/zsh-completions && \ 
    git clone https://github.com/ptavares/zsh-direnv.git $ZSH_PLUGINS_DIR/zsh-direnv 
RUN sed -i "s/plugins=(git)/plugins=(git zsh-syntax-highlighting zsh-completions zsh-direnv)/g" $HOME/.zshrc && \
    sed -i 's/ZSH_THEME=\"robbyrussell\"/ZSH_THEME=\"afowler\"/g' $HOME/.zshrc

########################################## Build dependencies ###########################################
# native
RUN sudo apt install -y libvulkan-dev vulkan-tools libegl1 libxext6 pkg-config libseat-dev libudev-dev

# rust
RUN sh -c "$(curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs)" -- -y && \
    echo "source $HOME/.cargo/env" >> $HOME/.zshrc

# node
#ENV PNPM_STORE=$WORKSPACE/.pnpm-store
RUN zsh -c "$(curl -fsSL https://get.pnpm.io/install.sh)"
RUN source $HOME/.zshrc && pnpm env use --global latest
RUN echo "alias p='pnpm'" >> $HOME/.zshrc

# protobuf
RUN cargo install protoc-gen-prost protoc-gen-prost-serde protoc-gen-tonic protoc-gen-prost-crate

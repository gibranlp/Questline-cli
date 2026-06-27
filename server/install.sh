#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# install.sh — instala y configura el servidor de Questline en Linux/macOS
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

INSTALL_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/questline"
BASE_URL="https://github.com/gibranlp/Questline-cli/releases/latest/download"

BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
DIM='\033[2m'
RESET='\033[0m'

echo ""
echo -e "  ${BOLD}${CYAN}QUESTLINE${RESET} — Installer"
echo -e "  ${DIM}────────────────────────────────────────${RESET}"

# ── Detectar OS — Linux y macOS sí, Windows que use el .ps1 ──────────────────
OS_RAW="$(uname -s)"
case "${OS_RAW}" in
  Linux*)  OS="linux"  ;;
  Darwin*) OS="macos"  ;;
  *)
    echo -e "  ${RED}Error: Unsupported OS: ${OS_RAW}${RESET}"
    echo -e "  For Windows, use: irm https://raw.githubusercontent.com/gibranlp/Questline-cli/main/server/install.ps1 | iex"
    exit 1
    ;;
esac

# ── Detectar arquitectura — x86_64 y ARM64/aarch64, los raros se van con error ──
ARCH_RAW="$(uname -m)"
case "${ARCH_RAW}" in
  x86_64|amd64)
    ARCH="x86_64"
    ;;
  arm64|aarch64)
    if [ "${OS}" = "macos" ]; then
      ARCH="arm64"
    else
      ARCH="aarch64"
    fi
    ;;
  *)
    echo -e "  ${RED}Error: Unsupported architecture: ${ARCH_RAW}${RESET}"
    exit 1
    ;;
esac

BINARY="questline-${OS}-${ARCH}"
DOWNLOAD_URL="${BASE_URL}/${BINARY}"

echo -e "  Platform  : ${OS}/${ARCH}"
echo -e "  Binary    : ${BINARY}"
echo -e "  Install   : ${INSTALL_DIR}/questline"
echo -e "  Config    : ${CONFIG_DIR}"
echo ""

# Crear directorios de instalación y config — si ya existen no pasa nada
mkdir -p "${INSTALL_DIR}"
mkdir -p "${CONFIG_DIR}"

# ── Helpers de la barra de progreso — pura cosmética pero se ve chido ─────────
_file_bytes() {
  if [ "${OS}" = "macos" ]; then
    stat -f%z "$1" 2>/dev/null || echo 0
  else
    stat -c%s "$1" 2>/dev/null || echo 0
  fi
}

_draw_bar() {
  local pct="$1" recv="$2" total="$3"
  local bar_width=40 i filled=0 filled_str="" empty_str=""

  filled=$(( bar_width * pct / 100 ))
  [ "$filled" -gt "$bar_width" ] && filled=$bar_width

  for ((i=0; i<filled; i++)); do
    if [ "$i" -eq $(( filled - 1 )) ] && [ "$pct" -lt 100 ]; then
      filled_str+=">"
    else
      filled_str+="="
    fi
  done
  for ((i=filled; i<bar_width; i++)); do empty_str+=" "; done

  local recv_mb size_str
  recv_mb="$(awk "BEGIN{printf \"%.1f\", ${recv}/1048576}")"
  if [ "${total}" -gt 0 ]; then
    local total_mb
    total_mb="$(awk "BEGIN{printf \"%.1f\", ${total}/1048576}")"
    size_str="${recv_mb} MB / ${total_mb} MB"
  else
    size_str="${recv_mb} MB"
  fi

  printf "\r  ${CYAN}[%-${bar_width}s]${RESET} ${BOLD}%3d%%${RESET}  ${DIM}%s${RESET}  " \
    "${filled_str}${empty_str}" "${pct}" "${size_str}"
}

# ── Descarga — usa curl si existe, si no wget; sin ninguno de los dos, adiós ──
if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
  echo -e "  ${RED}Error: curl or wget is required but neither was found.${RESET}"
  exit 1
fi

TMP_FILE="$(mktemp)"
trap 'rm -f "${TMP_FILE}"' EXIT

# Obtener el tamaño del archivo de antemano para calcular el porcentaje de descarga
TOTAL_BYTES=0
if command -v curl >/dev/null 2>&1; then
  TOTAL_BYTES="$(curl -fsSLI "${DOWNLOAD_URL}" 2>/dev/null \
    | grep -i "^content-length:" | tail -1 | awk '{print $2}' | tr -d '\r\n')" || true
fi
TOTAL_BYTES="${TOTAL_BYTES:-0}"

echo -e "  ${DIM}Downloading ${BOLD}${BINARY}${RESET}${DIM}...${RESET}"

# Lanzar descarga en background para poder dibujar la barra mientras tanto
DL_PID=""
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "${DOWNLOAD_URL}" -o "${TMP_FILE}" &
  DL_PID=$!
else
  wget -q "${DOWNLOAD_URL}" -O "${TMP_FILE}" &
  DL_PID=$!
fi

# Polling del tamaño del archivo cada 200ms para actualizar la barra — no es elegante pero jala
_draw_bar 0 0 "${TOTAL_BYTES}"
while kill -0 "${DL_PID}" 2>/dev/null; do
  RECV="$(_file_bytes "${TMP_FILE}")"
  if [ "${TOTAL_BYTES}" -gt 0 ]; then
    PCT=$(( RECV * 100 / TOTAL_BYTES ))
    [ "$PCT" -gt 100 ] && PCT=100
  else
    PCT=0
  fi
  _draw_bar "${PCT}" "${RECV}" "${TOTAL_BYTES}"
  sleep 0.2
done

if ! wait "${DL_PID}"; then
  printf "\n"
  echo -e "  ${RED}Error: Download failed.${RESET}"
  echo -e "  URL: ${DOWNLOAD_URL}"
  exit 1
fi

FINAL_BYTES="$(_file_bytes "${TMP_FILE}")"
_draw_bar 100 "${FINAL_BYTES}" "${FINAL_BYTES}"
printf "\n"
echo -e "  ${GREEN}✓ Download complete${RESET}"

# ── Instalar el binario — chmod, mv y listo ───────────────────────────────────
chmod +x "${TMP_FILE}"
mv "${TMP_FILE}" "${INSTALL_DIR}/questline"

echo -e "  ${GREEN}✓ Installed${RESET} → ${INSTALL_DIR}/questline"

# ── PATH setup — detecta el shell y agrega ~/.local/bin al perfil correcto ────
if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
  echo ""
  echo -e "  ${YELLOW}Setting up PATH...${RESET}"

  SHELL_NAME="$(basename "${SHELL:-}")"
  SHELL_CONFIG=""
  FISH_CONFIG="${HOME}/.config/fish/config.fish"
  IS_FISH=false

  case "${SHELL_NAME}" in
    zsh)
      if [ -f "${HOME}/.zshrc" ]; then
        SHELL_CONFIG="${HOME}/.zshrc"
      else
        SHELL_CONFIG="${HOME}/.zprofile"
      fi
      ;;
    bash)
      if [ "${OS}" = "macos" ] && [ -f "${HOME}/.bash_profile" ]; then
        SHELL_CONFIG="${HOME}/.bash_profile"
      elif [ -f "${HOME}/.bashrc" ]; then
        SHELL_CONFIG="${HOME}/.bashrc"
      else
        SHELL_CONFIG="${HOME}/.profile"
      fi
      ;;
    fish)
      IS_FISH=true
      SHELL_CONFIG="${FISH_CONFIG}"
      mkdir -p "$(dirname "${FISH_CONFIG}")"
      ;;
    ksh|mksh)
      SHELL_CONFIG="${HOME}/.kshrc"
      ;;
    *)
      SHELL_CONFIG="${HOME}/.profile"
      ;;
  esac

  PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'
  FISH_LINE='fish_add_path "$HOME/.local/bin"'

  if [ "$IS_FISH" = true ]; then
    if [ -f "${SHELL_CONFIG}" ] && grep -qF 'fish_add_path' "${SHELL_CONFIG}" 2>/dev/null && grep -qF '.local/bin' "${SHELL_CONFIG}" 2>/dev/null; then
      echo -e "  ${DIM}PATH already configured in ${SHELL_CONFIG}${RESET}"
    else
      printf '\n# Added by Questline installer\n%s\n' "${FISH_LINE}" >> "${SHELL_CONFIG}"
      echo -e "  ${GREEN}Added to${RESET} ${SHELL_CONFIG}"
      echo -e "  ${DIM}Run: source ${SHELL_CONFIG}${RESET}"
    fi
  elif [ -n "${SHELL_CONFIG}" ]; then
    if [ -f "${SHELL_CONFIG}" ] && grep -qF '.local/bin' "${SHELL_CONFIG}" 2>/dev/null; then
      echo -e "  ${DIM}PATH already configured in ${SHELL_CONFIG}${RESET}"
    else
      printf '\n# Added by Questline installer\n%s\n' "${PATH_LINE}" >> "${SHELL_CONFIG}"
      echo -e "  ${GREEN}Added to${RESET} ${SHELL_CONFIG}"
      echo -e "  ${DIM}Run: source ${SHELL_CONFIG}  (or open a new terminal)${RESET}"
    fi
  else
    echo -e "  ${YELLOW}Could not detect shell config file.${RESET}"
    echo -e "  Add this line manually to your shell profile:"
    echo ""
    echo -e "    ${CYAN}${PATH_LINE}${RESET}"
    echo ""
  fi
fi

# ── Lanzar Questline — si stdin es pipe (curl | bash) se redirige de /dev/tty ──
export PATH="${INSTALL_DIR}:${PATH}"

echo ""
echo -e "  ${BOLD}${GREEN}Installation complete!${RESET} Starting Questline..."
echo -e "  ${DIM}────────────────────────────────────────${RESET}"
echo ""

# En macOS con kqueue no se puede usar /dev/tty desde pipe — hay que correrlo a mano
if [ -t 0 ]; then
  exec "${INSTALL_DIR}/questline"
elif [ "${OS}" != "macos" ] && [ -e /dev/tty ]; then
  exec "${INSTALL_DIR}/questline" < /dev/tty
else
  echo -e "  Run ${CYAN}questline${RESET} to begin your journey."
  echo ""
fi
